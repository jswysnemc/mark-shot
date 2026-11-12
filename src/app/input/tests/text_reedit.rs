//! 【文字交互测试】【选择与编辑】已有文字单击选择、拖动移动，只有双击进入正文编辑

use super::*;
use crate::{tools::Tool, types::MouseButton};

/// 【文字交互测试】【样本创建】按 tool 返回具有一段未选择文字的编辑器与正文命中点
fn fixture(tool: Tool) -> (EditorState, (f64, f64)) {
    let mut state = editor();
    state.selection.zone = tiny_skia::Rect::from_xywh(0.0, 0.0, 640.0, 480.0);
    super::text::add_text(&mut state);
    state.selected_tool = tool;
    state.select_annotation_index(None);
    let bounds = state.annotations[0].bbox;
    let point = (
        f64::from(bounds.left() + bounds.width() / 2.0),
        f64::from(bounds.top() + bounds.height() / 2.0),
    );
    (state, point)
}

/// 【文字交互测试】【鼠标按钮】按 pressed 交付 state 当前指针的左键事件，无返回值
fn button(state: &mut EditorState, pressed: bool) {
    super::super::handle_pointer_button(state, MouseButton::Left, pressed, &mut 0);
}

/// 【文字交互测试】【单双击】文字和选择工具的首次完整点击不编辑，第二次同位置点击进入同一文字对象
#[test]
fn single_click_selects_and_double_click_edits_existing_text() {
    for tool in [Tool::Pick, Tool::Text] {
        let (mut state, point) = fixture(tool);
        pointer(&mut state, point.0, point.1, &mut 0);
        button(&mut state, true);
        button(&mut state, false);
        assert!(
            state.text_session.is_none(),
            "single click opened the text editor"
        );
        assert_eq!(state.selected_annotation_index(), Some(0));
        assert!(state.undo_stack.is_empty());
        button(&mut state, true);
        button(&mut state, false);
        assert_eq!(state.text_session.as_ref().unwrap().original_id, Some(7));
        assert!(state.ann_drag.is_none());
    }
}

/// 【文字交互测试】【拖动取消双击】拖动超出阈值又回到起点后，下一次单击仍只选择，不误入正文编辑
#[test]
fn dragging_out_and_back_cancels_the_double_click_candidate() {
    for tool in [Tool::Pick, Tool::Text] {
        let (mut state, point) = fixture(tool);
        let original = state.annotations[0].clone();
        pointer(&mut state, point.0, point.1, &mut 0);
        button(&mut state, true);
        pointer(&mut state, point.0 + 40.0, point.1 + 20.0, &mut 0);
        assert!(state.text_session.is_none());
        assert!(state.pending.is_some());
        pointer(&mut state, point.0, point.1, &mut 0);
        button(&mut state, false);
        assert_eq!(state.annotations[0].bbox, original.bbox);
        button(&mut state, true);
        button(&mut state, false);
        assert!(state.text_session.is_none());
    }
}
