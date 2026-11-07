//! 【框选翻译】【区域像素】仅保存选区原始片段，退出逐字节恢复，不复制整个工作区

use crate::{editor::EditorState, renderer, types::Capture};
use std::borrow::Cow;
use tiny_skia::{BlendMode, IntRect, Pixmap, PixmapPaint, Rect, Transform};

#[cfg(test)]
#[path = "pixels_tests.rs"]
mod tests;

struct Patch {
    index: Option<usize>,
    rect: IntRect,
    original: Pixmap,
    origin: (f32, f32),
    scale: (f32, f32),
}

pub struct Region {
    pub original: Pixmap,
    pub bounds: Rect,
    pub origin: (f32, f32),
    pub scale: f32,
    patches: Vec<Patch>,
}

impl Region {
    /// 【框选翻译】【区域保存】从 state 的 region 保存原始像素与本地预览片段，返回精确映射，不包含标注
    pub fn capture(state: &EditorState, region: Rect) -> Option<Self> {
        let (original, origin, scale) = if let Some(file) = &state.file_canvas {
            let crop = file.crop(region)?;
            (crop.pixels, crop.global_origin, crop.scale)
        } else {
            let (pixels, origin, scale) =
                renderer::composite_precise(&state.captures, &state.placements, region, None)?;
            (pixels, (origin.0 as f32, origin.1 as f32), scale)
        };
        let bounds = Rect::from_xywh(
            origin.0,
            origin.1,
            original.width() as f32 / scale,
            original.height() as f32 / scale,
        )?;
        let mut patches = Vec::new();
        if let Some(file) = &state.file_canvas {
            let source = file.source_region(bounds)?;
            patches.push(Patch {
                index: None,
                rect: source.bounds,
                original: file.original.clone_rect(source.bounds)?,
                origin: source.global_origin,
                scale: (source.scale, source.scale),
            });
        }
        for (index, (capture, placement)) in
            state.captures.iter().zip(&state.placements).enumerate()
        {
            let monitor = Rect::from_xywh(
                placement.position.0 as f32,
                placement.position.1 as f32,
                placement.size.0 as f32,
                placement.size.1 as f32,
            )?;
            let Some(overlap) = bounds.intersect(&monitor) else {
                continue;
            };
            let sx = capture.pixmap.width() as f32 / monitor.width();
            let sy = capture.pixmap.height() as f32 / monitor.height();
            let pixels = Rect::from_ltrb(
                (overlap.left() - monitor.left()) * sx,
                (overlap.top() - monitor.top()) * sy,
                (overlap.right() - monitor.left()) * sx,
                (overlap.bottom() - monitor.top()) * sy,
            )?;
            let (x, y, w, h) =
                renderer::rect_bounds(&pixels, capture.pixmap.width(), capture.pixmap.height())?;
            let rect = IntRect::from_xywh(x as i32, y as i32, w, h)?;
            patches.push(Patch {
                index: Some(index),
                rect,
                original: capture.pixmap.clone_rect(rect)?,
                origin: (
                    monitor.left() + x as f32 / sx,
                    monitor.top() + y as f32 / sy,
                ),
                scale: (sx, sy),
            });
        }
        Some(Self {
            original,
            bounds,
            origin,
            scale,
            patches,
        })
    }

    /// 【框选翻译】【显示写回】按 image 写入 captures 和可选 file；空值逐字节恢复原片段，不改变区域外像素
    pub fn apply(
        &self,
        captures: &mut [Capture],
        file: Option<&mut crate::editor::file_canvas::FileCanvas>,
        image: Option<&Pixmap>,
    ) {
        let mut file = file;
        for patch in &self.patches {
            if patch.index.is_some()
                && image.is_some()
                && let Some(file) = file.as_ref()
            {
                let mut preview = patch.original.clone();
                preview.fill(tiny_skia::Color::from_rgba8(28, 31, 34, 255));
                let scale = file.viewport.width() / file.original.width() as f32;
                let transform = Transform::from_scale(scale * patch.scale.0, scale * patch.scale.1)
                    .post_translate(
                        (file.viewport.left() - patch.origin.0) * patch.scale.0,
                        (file.viewport.top() - patch.origin.1) * patch.scale.1,
                    );
                preview.draw_pixmap(
                    0,
                    0,
                    file.original.as_ref(),
                    &PixmapPaint {
                        quality: tiny_skia::FilterQuality::Bilinear,
                        ..PixmapPaint::default()
                    },
                    transform,
                    None,
                );
                captures[patch.index.unwrap()]
                    .pixmap
                    .replace_pixels(&preview, (patch.rect.x(), patch.rect.y()));
                continue;
            }
            let pixels = if let Some(image) = image {
                let transform =
                    Transform::from_scale(patch.scale.0 / self.scale, patch.scale.1 / self.scale)
                        .post_translate(
                            (self.origin.0 - patch.origin.0) * patch.scale.0,
                            (self.origin.1 - patch.origin.1) * patch.scale.1,
                        );
                if transform.is_identity()
                    && image.width() == patch.original.width()
                    && image.height() == patch.original.height()
                {
                    Cow::Borrowed(image)
                } else {
                    let mut pixels = patch.original.clone();
                    pixels.draw_pixmap(
                        0,
                        0,
                        image.as_ref(),
                        &PixmapPaint {
                            blend_mode: BlendMode::Source,
                            quality: tiny_skia::FilterQuality::Bilinear,
                            ..PixmapPaint::default()
                        },
                        transform,
                        None,
                    );
                    Cow::Owned(pixels)
                }
            } else {
                Cow::Borrowed(&patch.original)
            };
            // 1. 【框选翻译】【原件版本】屏幕预览写入冻结片段，文件原件创建新版本，既有只读映射不接受写入
            if let Some(index) = patch.index {
                captures[index]
                    .pixmap
                    .replace_pixels(&pixels, (patch.rect.x(), patch.rect.y()));
            } else {
                file.as_mut()
                    .unwrap()
                    .original
                    .replace_pixels(&pixels, (patch.rect.x(), patch.rect.y()));
            }
        }
        if let Some(file) = file {
            file.original.release_pages();
        }
    }
}
