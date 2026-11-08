//! 【光标放大区】【阶段判断】沿用原版初始选择、区域调整和取色职责，静止编辑不常驻圆形放大区

use super::EditorState;
use crate::{
    tools::Tool,
    types::{Mode, SelectionHandle},
};

/// 【光标放大区】【显示判断】按 state 的工具、选区手势及弹层返回是否显示；开关只控制支持的阶段，取色保留独立开关优先级
pub fn visible(state: &EditorState) -> bool {
    if crate::app::canvas_navigation::input::owns_pointer(state)
        || state.mode == Mode::Ocr
        || state.selection_translation.busy()
        || state.selection_translation.popup.is_some()
        || state.selection_size.interacting()
        || state.text_context_menu.is_some()
        || state.external_panel.kind.is_some()
        || state.font_panel.open
        || state.color_popover.open
        || state.model_popover.open
        || state.marker_popover.open
        || state.ruler_popover.open
        || state.number_popover.open
        || state.arrow_popover.open
        || state.line_popover.open
        || state.filter_popover.open
        || state.color_inspector.panel.is_some()
    {
        return false;
    }
    if state.picking() {
        return true;
    }
    !state.magnifier.hidden
        && !state.fullscreen_scope
        && state.selected_tool == Tool::Selection
        && (state.selection.zone.is_none()
            || state.input.mouse_down
                && (state.input.drag_start.is_some()
                    || state.selection.active_handle != SelectionHandle::None))
}
