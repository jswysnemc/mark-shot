//! 【截图历史测试】【手势保护】通过真实按键与指针分派覆盖未提交草稿和已有标注拖动

use super::{editor, key, pointer};
use crate::app::input::{handle_pointer_button, select_tool};
use crate::editor::EditorState;
use crate::tools::Tool;
use crate::types::MouseButton;
use crate::ui::toast::ToastKind;

/// 【截图历史测试】【交接断言】对 state 发送 Ctrl+H；dirty 返回重绘范围，忙碌会话不启动窗口或显示启动失败
fn expect_ignored(state: &mut EditorState, dirty: &mut u32) {
    let history_size = state.undo_stack.len();
    key(state, "Ctrl+H", dirty);
    assert!(!state.cancel);
    assert_eq!(state.undo_stack.len(), history_size);
    assert!(
        !state
            .toasts
            .items
            .iter()
            .any(|toast| toast.kind == ToastKind::HistoryFailed),
        "the ignored shortcut attempted to launch a helper"
    );
}

/// 【截图历史测试】【旧工具草稿】旧绘图工具不设置 mouse_down 或 tool_active，实际草稿仍必须阻止窗口交接
#[test]
fn pending_legacy_drawing_gestures_ignore_history_shortcuts() {
    for tool in [
        Tool::Pen,
        Tool::Highlighter,
        Tool::Rectangle,
        Tool::Circle,
        Tool::NumeratedArrow,
    ] {
        let mut state = editor();
        let mut dirty = 0;
        select_tool(&mut state, tool, &mut dirty);
        pointer(&mut state, 140.0, 130.0, &mut dirty);
        handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
        pointer(&mut state, 220.0, 180.0, &mut dirty);
        assert!(state.pending.is_some(), "{tool:?}");
        assert!(!state.input.mouse_down && !state.tool_active, "{tool:?}");
        let before = state.pending.clone();
        expect_ignored(&mut state, &mut dirty);
        assert_eq!(state.pending, before, "{tool:?}");
    }
}

/// 【截图历史测试】【已有标注拖动】已有图形拖动携带原始快照，Ctrl+H 不结束拖动或改变标注快照
#[test]
fn existing_annotation_drags_ignore_history_shortcuts() {
    let mut state = editor();
    let mut dirty = 0;
    select_tool(&mut state, Tool::Rectangle, &mut dirty);
    pointer(&mut state, 140.0, 130.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    pointer(&mut state, 220.0, 180.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    select_tool(&mut state, Tool::Pick, &mut dirty);
    pointer(&mut state, 180.0, 155.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    pointer(&mut state, 190.0, 165.0, &mut dirty);
    assert!(state.ann_drag.is_some());
    assert!(!state.input.mouse_down && !state.tool_active);
    let before = state.annotations.clone();
    let drag_origin = state.ann_drag.as_ref().unwrap().orig.clone();
    expect_ignored(&mut state, &mut dirty);
    assert_eq!(state.annotations, before);
    assert_eq!(state.ann_drag.as_ref().unwrap().orig, drag_origin);
}

/// 【截图历史测试】【选区拖动】选区工具只设置按下标志，尚无标注草稿时也必须忽略历史快捷键
#[test]
fn selection_drags_with_no_annotation_draft_ignore_history_shortcuts() {
    let mut state = editor();
    let mut dirty = 0;
    select_tool(&mut state, Tool::Selection, &mut dirty);
    pointer(&mut state, 140.0, 130.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    pointer(&mut state, 220.0, 180.0, &mut dirty);
    assert!(state.input.mouse_down && state.tool_active);
    assert!(state.pending.is_none() && state.ann_drag.is_none());
    expect_ignored(&mut state, &mut dirty);
}
