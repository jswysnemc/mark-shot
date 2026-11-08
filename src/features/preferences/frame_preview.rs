//! 【设置窗口】【外框预览】使用实际导出算法合成固定示例，缓存参数不变的图像

use super::{catalog::ENTRIES, layout::rect, window::Preferences};
use crate::{
    config::export::ImageFrame,
    renderer::{Painter, text::TextStyle},
    theme::window::Palette,
};
use tiny_skia::{Color, FilterQuality, Pixmap, PixmapPaint, Rect, Transform};

#[derive(Default)]
pub(super) struct Preview {
    options: Option<ImageFrame>,
    image: Option<Pixmap>,
}

/// 【设置窗口】【预览参数】从 state 读取已保存值并叠加合法草稿，返回实际导出配置，不写文件
pub(super) fn options(state: &Preferences) -> ImageFrame {
    let mut values = state.values["export"]["image_frame"].clone();
    if let Some((index, editor)) = &state.editing {
        let entry = &ENTRIES[*index];
        if entry.section == "export.image_frame"
            && let Ok(value) = entry.parse(&editor.text())
        {
            values[entry.key] = value;
        }
    }
    ImageFrame::from_value(&values)
}

impl Preview {
    /// 【设置窗口】【预览缓存】按 options 更新合成图，参数未变化时复用已有像素，无返回值
    pub fn update(&mut self, options: ImageFrame) {
        if self.options == Some(options) {
            return;
        }
        self.options = Some(options);
        self.image = render(options);
    }

    /// 【设置窗口】【预览展示】按 bounds 和 chinese 绘制透明棋盘与缩放后的导出图，painter 处理裁切和缩放
    pub fn draw(&self, painter: &mut Painter<'_>, bounds: Rect, chinese: bool) {
        let palette = Palette::current();
        painter.text(
            rect(bounds.left(), bounds.top(), bounds.width(), 20.0),
            if chinese {
                "实际导出预览"
            } else {
                "Export preview"
            },
            TextStyle::new(11.0, palette.muted),
        );
        let area = rect(
            bounds.left(),
            bounds.top() + 24.0,
            bounds.width(),
            bounds.height() - 48.0,
        );
        painter.fill_rounded(area, 6.0, palette.window);
        for row in 0..(area.height() / 8.0).ceil() as usize {
            for column in 0..(area.width() / 8.0).ceil() as usize {
                if (row + column) % 2 == 0 {
                    let left = area.left() + column as f32 * 8.0;
                    let top = area.top() + row as f32 * 8.0;
                    painter.fill_rounded(
                        rect(
                            left,
                            top,
                            (area.right() - left).min(8.0),
                            (area.bottom() - top).min(8.0),
                        ),
                        0.0,
                        palette.alternate,
                    );
                }
            }
        }
        if let Some(image) = &self.image {
            let scale = ((area.width() - 16.0) / image.width() as f32)
                .min((area.height() - 12.0) / image.height() as f32);
            let left = area.left() + (area.width() - image.width() as f32 * scale) / 2.0;
            let top = area.top() + (area.height() - image.height() as f32 * scale) / 2.0;
            let transform = painter
                .transform()
                .pre_concat(Transform::from_row(scale, 0.0, 0.0, scale, left, top));
            painter.canvas().draw_pixmap(
                0,
                0,
                image.as_ref(),
                &PixmapPaint {
                    quality: FilterQuality::Bilinear,
                    ..Default::default()
                },
                transform,
                None,
            );
            let enabled = self.options.is_some_and(|frame| frame.enabled);
            let label = format!(
                "{} · {} × {} px",
                if enabled {
                    if chinese {
                        "已添加外框"
                    } else {
                        "Frame enabled"
                    }
                } else if chinese {
                    "原图 · 外框关闭"
                } else {
                    "Original · frame off"
                },
                image.width(),
                image.height()
            );
            painter.text(
                rect(bounds.left(), bounds.bottom() - 20.0, bounds.width(), 20.0),
                &label,
                TextStyle::new(10.0, palette.muted).center(),
            );
        }
    }
}

/// 【设置窗口】【示例图像】无参数，绘制固定 480×300 风景示例，返回独立像素图，不读取用户截图
fn sample() -> Pixmap {
    let mut image = Pixmap::new(480, 300).unwrap();
    let mut painter = Painter::new(&mut image);
    painter.fill_rounded_gradient(
        rect(0.0, 0.0, 480.0, 300.0),
        0.0,
        Color::from_rgba8(186, 215, 222, 255),
        Color::from_rgba8(232, 229, 211, 255),
    );
    painter.fill_circle(362.0, 72.0, 26.0, Color::from_rgba8(255, 246, 215, 255));
    for (points, color) in [
        (
            [
                (0.0, 194.0),
                (122.0, 81.0),
                (272.0, 226.0),
                (386.0, 126.0),
                (480.0, 205.0),
            ],
            Color::from_rgba8(114, 151, 156, 255),
        ),
        (
            [
                (0.0, 236.0),
                (91.0, 167.0),
                (238.0, 252.0),
                (356.0, 186.0),
                (480.0, 256.0),
            ],
            Color::from_rgba8(66, 112, 122, 255),
        ),
    ] {
        let mut path = tiny_skia::PathBuilder::new();
        path.move_to(0.0, 300.0);
        for (left, top) in points {
            path.line_to(left, top);
        }
        path.line_to(480.0, 300.0);
        path.close();
        painter.fill_path(&path.finish().unwrap(), color);
    }
    image
}

/// 【设置窗口】【真实合成】按 options 调用正式外框渲染器，返回 RGBA 预乘缓存，分配失败返回空值
fn render(options: ImageFrame) -> Option<Pixmap> {
    let source = sample();
    let pixels: Vec<u32> = source
        .pixels()
        .iter()
        .map(|pixel| u32::from_be_bytes([pixel.alpha(), pixel.red(), pixel.green(), pixel.blue()]))
        .collect();
    let view = shot_image::ImageView::new(
        &pixels,
        source.width() as i32,
        source.height() as i32,
        source.width() as usize,
    )
    .ok()?;
    let result = shot_image::export::render(view, &options.options()).ok()?;
    let mut image = Pixmap::new(result.width() as u32, result.height() as u32)?;
    for (output, pixel) in image
        .data_mut()
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip(result.pixels())
    {
        let [alpha, red, green, blue] = pixel.to_be_bytes();
        output.copy_from_slice(&[red, green, blue, alpha]);
    }
    Some(image)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【设置窗口】【外框回归】无参数，关闭时保留原图，启用后尺寸、透明留白及阴影实际变化
    #[test]
    fn preview_uses_export_pixels_and_padding() {
        let frame = ImageFrame::default();
        let original = render(frame).unwrap();
        assert_eq!(original, sample());
        let framed = render(ImageFrame {
            enabled: true,
            ..frame
        })
        .unwrap();
        assert_eq!((framed.width(), framed.height()), (704, 524));
        assert_eq!(framed.pixel(0, 0).unwrap().alpha(), 0);
        let transparent = render(ImageFrame {
            enabled: true,
            shadow_opacity: 0.0,
            ..frame
        })
        .unwrap();
        assert_ne!(framed, transparent);
    }
}
