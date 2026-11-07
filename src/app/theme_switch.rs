//! 【截图界面】【主题切换】刷新显示背景和面板缓存，不修改标注、选择和历史事务

use crate::{
    editor::{EditorState, dirty::mark_all_dirty},
    ui::panel::UiPanel,
};

/// 【截图界面】【切换刷新】在 state 中切换主题并清理界面缓存，dirty_mask 返回重绘输出；保存失败只记录错误
pub(super) fn toggle(state: &mut EditorState, dirty_mask: &mut u32) {
    if let Err(error) = crate::theme::toggle() {
        log::error!("【截图界面】【主题切换】无法保存主题: {error}");
        return;
    }
    refresh(state, dirty_mask);
}

/// 【截图界面】【主题刷新】清理 state 的界面缓存，dirty_mask 返回重绘输出，不修改正文或配置
pub(super) fn refresh(state: &mut EditorState, dirty_mask: &mut u32) {
    super::file_canvas_theme::refresh(state, crate::theme::Mode::current());
    // 1. 【截图界面】【缓存失效】图形与原图缓存保留，所有面板重新使用当前主题绘制
    state.toolbar.drawn_look = None;
    state.action_toolbar.drawn_look = None;
    state.settings_panel.panel_pixmap = None;
    state.settings_panel.set_dirty();
    state.font_panel.pixmap = None;
    state.font_panel.set_dirty();
    state.model_popover.pixmap = None;
    state.model_popover.set_dirty();
    state.color_popover.set_dirty();
    state.external_panel.dirty = true;
    if let Some(panel) = &mut state.selection_size.panel {
        panel.dirty = true;
    }
    if let Some(popup) = &mut state.selection_size.popup {
        popup.dirty = true;
    }
    for panel in [
        &mut state.marker_popover,
        &mut state.ruler_popover,
        &mut state.number_popover,
        &mut state.arrow_popover,
        &mut state.line_popover,
        &mut state.filter_popover,
    ] {
        panel.pixmap = None;
    }
    // 2. 【截图界面】【完整输出】已有文字或画笔局部损坏不能缩小主题刷新，记录每个输出的完整本地范围
    for index in 0..state.placements.len() {
        let size = state.placements[index].size;
        if let Some(rect) = tiny_skia::Rect::from_xywh(0.0, 0.0, size.0 as f32, size.1 as f32) {
            state.damage_local(index, rect);
        }
    }
    mark_all_dirty(dirty_mask, state.placements.len());
}

#[cfg(test)]
#[path = "theme_switch_tests.rs"]
mod tests;
