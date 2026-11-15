//! 【图片窗口】【合并栏绘制】工具与窗口按钮共用顶部背景，不再绘制独立标题栏

use super::{Hit, controls, layout};
use crate::{editor::EditorState, renderer::Painter};
use tiny_skia::Rect;

/// 【图片窗口】【合并栏绘制】按state与monitor绘制横贯窗口的工具栏和窗口按钮，不修改原图或导出缓冲
pub(crate) fn draw(state: &mut EditorState, monitor: usize) {
    if !state.image_window.enabled || monitor != 0 {
        return;
    }
    let size = (
        state.placements[0].size.0 as u32,
        state.placements[0].size.1 as u32,
    );
    let height = layout::bar_height(state.toolbar.size.1);
    let scale = state.placements[0].scale;
    let mut painter = Painter::new(&mut state.canvas[0])
        .with_scale(scale)
        .with_icons(&state.icons_cache);
    // 1. 【图片窗口】【统一背景】工具直接嵌入整行背景，取消窗口标题及独立工具条圆角外框
    painter.fill_rounded(
        Rect::from_xywh(0.0, 0.0, size.0 as f32, height).unwrap(),
        0.0,
        crate::config::get().toolbar.background.get(),
    );
    crate::ui::toolbar::draw_toolbar(&mut painter, &mut state.toolbar);
    // 2. 【图片窗口】【窗口操作】右侧三个按钮与工具使用相同尺寸和配色，画布直接衔接顶部栏
    for (index, button) in [Hit::Close, Hit::Maximize, Hit::Minimize]
        .into_iter()
        .enumerate()
    {
        controls::draw(
            &mut painter,
            layout::button(size, height, index),
            button,
            state.image_window.hovered == Some(button),
            state.image_window.pressed == Some(button),
        );
    }
    crate::ui::window_frame::draw(&mut state.canvas[0], scale);
}
