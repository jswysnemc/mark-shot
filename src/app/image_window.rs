//! 【图片编辑】【窗口画布】窗口调整只改变显示资源与视图，原件和标注坐标保持稳定

use crate::{
    backend::ScreenOverlay,
    editor::{EditorState, file_canvas::FileCanvas},
    types::{Capture, Placement},
};
use tiny_skia::Rect;

#[cfg(test)]
#[path = "image_window_tests.rs"]
mod tests;

/// 【图片编辑】【初始窗口】按 file 原图及 placement 输出请求窗口尺寸；overlay 确认后更新逻辑画布，返回协议结果
pub(super) fn prepare(
    overlay: &mut dyn ScreenOverlay,
    file: &mut FileCanvas,
    placement: &mut Placement,
    output: usize,
) -> crate::AppResult {
    let maximum = (placement.size.0 as f32 * 0.9, placement.size.1 as f32 * 0.9);
    let source = (file.original.width() as f32, file.original.height() as f32);
    let factor = (maximum.0 / source.0).min(maximum.1 / source.1).min(1.0);
    let requested = (
        (source.0 * factor).max(640.0_f32.min(maximum.0)).round() as u32,
        (source.1 * factor).max(360.0_f32.min(maximum.1)).round() as u32,
    );
    let window = overlay.configure_image_window(requested);
    overlay.present(&[output])?;
    if window && let Some(size) = overlay.image_window_size() {
        placement.position = (0, 0);
        placement.size = (size.0 as i32, size.1 as i32);
        let navigation = crate::features::canvas_navigation::model::Navigation::new(
            (file.original.width(), file.original.height()),
            size,
            true,
        )
        .ok_or("invalid editor window size")?;
        let fitted = navigation.projection().widget_rect;
        file.viewport = Rect::from_xywh(
            fitted.x as f32,
            fitted.y as f32,
            fitted.width as f32,
            fitted.height as f32,
        )
        .ok_or("invalid editor image viewport")?;
    }
    Ok(())
}

/// 【图片编辑】【画布调整】按 size 重建 state 的窗口资源，dirty 返回完整损坏；保留原件、选择、历史和当前缩放，无返回值
pub(super) fn resize(state: &mut EditorState, size: (u32, u32), dirty: &mut u32) {
    if state.file_canvas.is_none()
        || size.0 == 0
        || size.1 == 0
        || state.placements[0].size == (size.0 as i32, size.1 as i32)
    {
        return;
    }
    state.ensure_navigation();
    state.placements[0].size = (size.0 as i32, size.1 as i32);
    state.navigation.window = Rect::from_xywh(0.0, 0.0, size.0 as f32, size.1 as f32);
    if let Some(view) = &mut state.navigation.view {
        view.resize(size, true);
    }
    state.navigation.frames.fill(None);
    state.toolbar.user_position = None;
    state.action_toolbar.user_position = None;
    // 1. 【图片编辑】【资源替换】只重建窗口大小的显示缓冲，不缩放原件或改写标注几何
    let (canvas, dimmed, annotations) =
        super::init::build_layers_with_transparency(&state.placements, true);
    state.canvas = canvas;
    state.dimmed = dimmed;
    state.annotations_layer = annotations;
    // 2. 【图片编辑】【按需预览】变换视图直接采样原件，连续调整窗口无需额外缩放整图或反复回收原件页
    if state.canvas_transform().is_identity()
        && let Some(preview) = super::file_canvas_theme::preview(
            state.file_canvas.as_ref().unwrap(),
            &state.placements[0],
            crate::theme::Mode::current(),
        )
    {
        state.captures = vec![Capture {
            pixmap: preview,
            scale: state.placements[0].scale,
        }];
    }
    state.annotations_dirty = true;
    state.enable_region_rendering();
    super::input::refresh_panels(state, dirty);
    crate::editor::dirty::mark_all_dirty(dirty, state.placements.len());
}
