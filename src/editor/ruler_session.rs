//! 【尺子】【会话切换】管理草稿取消、测量模式与从选区进入后的返回流程

use crate::editor::dirty::{apply_damage_rects, mark_all_dirty, mark_dirty};
use crate::editor::{DamageZone, EditorState};
use crate::features::ruler::RulerMode;
use crate::tools::Tool;
use crate::types::{AnnotationShape, SelectionHandle};
use crate::ui::toast::ToastKind;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CancelResult {
    Unhandled,
    Canceled,
    ReturnToSelection,
}

/// 【尺子】【草稿取消】撤去 state 的未提交尺子，不删除已完成测量；dirty_mask 返回重绘显示器，返回是否取消
pub fn cancel_draft(state: &mut EditorState, dirty_mask: &mut u32) -> bool {
    if state.ann_drag.is_some()
        || !state
            .pending
            .as_ref()
            .is_some_and(|annotation| matches!(annotation.shape, AnnotationShape::Ruler { .. }))
    {
        return false;
    }
    crate::editor::drawing_gesture::cancel(state, dirty_mask);
    true
}

/// 【尺子】【模式切换】为 state 设置 mode 并取消旧草稿；announce 决定是否显示提示，dirty_mask 返回重绘显示器，无返回值
pub fn set_mode(state: &mut EditorState, mode: RulerMode, announce: bool, dirty_mask: &mut u32) {
    cancel_draft(state, dirty_mask);
    if state.tool_settings.ruler_mode == mode {
        return;
    }
    state.tool_settings.ruler_mode = mode;
    state.settings_panel.dirty = true;
    if announce {
        let text = match mode {
            RulerMode::Length => "Measure length",
            RulerMode::Angle => "Measure angle",
        };
        state.toasts.show_text(
            ToastKind::RulerMode,
            text.to_string(),
            &mut state.text.font_system,
        );
        mark_dirty(dirty_mask, state.input.pointer.monitor_idx);
    }
}

/// 【尺子】【取消或返回】在 state 的尺子工具中先取消草稿，再清除本次选区入口测量；dirty_mask 返回重绘显示器，返回应用需要执行的取消动作
pub fn cancel_or_leave(state: &mut EditorState, dirty_mask: &mut u32) -> CancelResult {
    if state.selected_tool != Tool::Ruler {
        return CancelResult::Unhandled;
    }
    if cancel_draft(state, dirty_mask) {
        return CancelResult::Canceled;
    }
    if state.ann_drag.is_some() {
        state.undo(dirty_mask);
        state.input.mouse_down = false;
        state.tool_active = false;
        return CancelResult::Canceled;
    }
    if !state.ruler.entered_from_selection {
        return CancelResult::Unhandled;
    }
    // 1. 【尺子】【返回选区】这个入口开始时没有标注，返回时清除本次全屏测量及其历史
    for annotation in &state.annotations {
        state.layer_damage_rects.push(annotation.damage_bbox(false));
        state
            .damage_rects
            .push(DamageZone::Global(annotation.damage_bbox(true)));
    }
    state.annotations.clear();
    state.undo_stack.clear();
    state.redo_stack.clear();
    state.select_annotation_index(None);
    state.ann_drag = None;
    state.pending = None;
    state.prev_pending = None;
    state.next_id = 0;
    state.annotations_dirty = true;
    state.ruler.entered_from_selection = false;
    state.laser = crate::features::laser_trace::LaserTrace::default();
    // 2. 【尺子】【返回选区】恢复选区工具，下一次 Escape 才关闭截图会话
    state.selection.zone = None;
    state.selection.set_drag(SelectionHandle::None, None, None);
    state.input.mouse_down = false;
    state.tool_active = false;
    mark_all_dirty(dirty_mask, state.placements.len());
    apply_damage_rects(state, dirty_mask);
    CancelResult::ReturnToSelection
}

/// 【尺子】【悬停清除】删除 state 的临时十字线与取色说明；dirty_mask 返回旧区域重绘，无返回值
pub fn clear_hover(state: &mut EditorState, dirty_mask: &mut u32) {
    if let Some(hover) = state.ruler.hover.take() {
        state.damage_rects.push(DamageZone::Global(hover.bounds));
        apply_damage_rects(state, dirty_mask);
    }
}
