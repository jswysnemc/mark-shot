//! 【图片编辑】【显示主题】只重建冻结预览，原件、标注和导出颜色不参与主题切换

use crate::{
    editor::{EditorState, file_canvas::FileCanvas},
    theme::Mode,
    types::{Placement, frozen_pixmap::FrozenPixmap},
};
use tiny_skia::{Pixmap, PixmapPaint, Transform};

/// 【图片编辑】【主题预览】按 file、placement 和 mode 生成显示像素，保留原始尺寸与透明度，不修改原件
pub(super) fn preview(
    file: &FileCanvas,
    placement: &Placement,
    mode: Mode,
) -> Option<FrozenPixmap> {
    let (width, height) = placement.px_size();
    let mut pixels = Pixmap::new(width, height)?;
    pixels.fill(crate::theme::canvas::background(mode).color());
    let factor = file.viewport.width() / file.original.width() as f32 * placement.scale;
    let transform = Transform::from_scale(factor, factor).post_translate(
        (file.viewport.left() - placement.position.0 as f32) * placement.scale,
        (file.viewport.top() - placement.position.1 as f32) * placement.scale,
    );
    file.original.draw_preview(
        &mut pixels,
        &PixmapPaint {
            quality: tiny_skia::FilterQuality::Bilinear,
            ..PixmapPaint::default()
        },
        transform,
    );
    file.original.release_pages();
    Some(FrozenPixmap::cached(pixels))
}

/// 【显示主题】【缓存刷新】按 mode 重建 state 的文件预览并通知原生背景同步；截图原件和所有标注缓存保持不变
pub(super) fn refresh(state: &mut EditorState, mode: Mode) {
    if let Some(file) = &state.file_canvas {
        for (capture, placement) in state.captures.iter_mut().zip(&state.placements) {
            if let Some(pixels) = preview(file, placement, mode) {
                capture.pixmap = pixels;
                // 1. 【显示主题】【原生同步】复用事件循环已有背景交接，不在输入处理中操作覆盖层
                state.selection_translation.background_dirty = true;
            } else {
                log::error!("【图片编辑】【主题预览】无法分配显示像素");
            }
        }
    }
    // 2. 【显示主题】【导航失效】缩放后的背景含显示底色，下一帧必须重采样；恒等视图保留旧键以识别复位过渡
    if !state.canvas_transform().is_identity() {
        state.navigation.frames.fill(None);
    }
}

#[cfg(test)]
#[path = "file_canvas_theme_tests.rs"]
mod tests;
