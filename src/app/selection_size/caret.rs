//! 【选区尺寸】【输入光标】字段聚焦时按原版周期闪烁，编辑结束后清除所有定时唤醒

use crate::editor::EditorState;
use std::time::Instant;

/// 【选区尺寸】【光标同步】按 state 和 now 同步字段焦点及到期相位，dirty 返回局部面板，无返回值
pub(in crate::app) fn tick(state: &mut EditorState, now: Instant, dirty: &mut u32) {
    let active = state.input.keyboard_focus.is_some()
        && state.selection_size.popup.is_none()
        && state
            .selection_size
            .panel
            .as_ref()
            .is_some_and(|panel| panel.rect.is_some() && panel.editing().is_some());
    let focus = state.selection_size.caret.focus(active, now);
    let phase = state.selection_size.caret.tick(now);
    if focus || phase {
        super::panels::damage(state, dirty);
    }
}

/// 【选区尺寸】【输入活动】按 state 恢复光标相位，dirty 返回需要重绘的字段，不改变周期截止，无返回值
pub(in crate::app) fn activity(state: &mut EditorState, dirty: &mut u32) {
    tick(state, Instant::now(), dirty);
    if state.selection_size.caret.activity() {
        super::panels::damage(state, dirty);
    }
}
