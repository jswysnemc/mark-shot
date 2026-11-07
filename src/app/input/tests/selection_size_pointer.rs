//! 【选区尺寸测试】【鼠标归属】应用入口验证多击、持有按钮、字段交接和跨输出坐标

use super::*;
use crate::{features::selection_size::ratio::Field, types::MouseButton};

/// 【选区尺寸测试】【活动字段】返回通过键盘开始编辑正号数值的截图，不预先建立鼠标点击序列
fn editing() -> EditorState {
    let mut state = super::selection_size::selected();
    key(&mut state, "Ctrl+R", &mut 0);
    super::super::handle_key(&mut state, None, Some("+120".into()), &mut 0);
    state
}

/// 【选区尺寸测试】【字段中心】按 state 返回当前宽度字段中心，不使用固定屏幕坐标
fn center(state: &EditorState) -> (f64, f64) {
    let rect = state
        .selection_size
        .panel
        .as_ref()
        .unwrap()
        .field_rect(Field::Width)
        .unwrap();
    (
        f64::from(rect.left() + rect.width() / 2.0),
        f64::from(rect.top() + rect.height() / 2.0),
    )
}

/// 【选区尺寸测试】【真实按钮】将 button 与 pressed 投递到 state 的应用入口，无返回值
fn button(state: &mut EditorState, button: MouseButton, pressed: bool) {
    super::super::handle_pointer_button(state, button, pressed, &mut 0);
}

/// 【选区尺寸测试】【双击三击】第二次点击仅选择数字段，第三次选择包括正号的全部正文
#[test]
fn double_and_triple_click_select_words_and_full_text() {
    let mut state = editing();
    let point = center(&state);
    pointer(&mut state, point.0, point.1, &mut 0);
    for pressed in [true, false] {
        button(&mut state, MouseButton::Left, pressed);
    }
    assert!(
        state.selection_size.panel.as_ref().unwrap().fields[0]
            .editor
            .selection_range()
            .is_none()
    );
    for pressed in [true, false] {
        button(&mut state, MouseButton::Left, pressed);
    }
    assert_eq!(
        state.selection_size.panel.as_ref().unwrap().fields[0]
            .editor
            .selected_text()
            .as_deref(),
        Some("120")
    );
    for pressed in [true, false] {
        button(&mut state, MouseButton::Left, pressed);
    }
    assert_eq!(
        state.selection_size.panel.as_ref().unwrap().fields[0]
            .editor
            .selected_text()
            .as_deref(),
        Some("+120")
    );
    assert_eq!(state.selection.zone.unwrap().width(), 300.0);
}

/// 【选区尺寸测试】【释放归属】左键持有字段拖选时，右键按下与释放不结束该拖选
#[test]
fn right_release_does_not_finish_a_left_field_drag() {
    let mut state = editing();
    let point = center(&state);
    pointer(&mut state, point.0, point.1, &mut 0);
    button(&mut state, MouseButton::Left, true);
    let anchor = state.selection_size.panel.as_ref().unwrap().fields[0]
        .editor
        .cursor;
    assert!(anchor > 0);
    for pressed in [true, false] {
        button(&mut state, MouseButton::Right, pressed);
    }
    assert_eq!(state.selection_size.pressed, Some(MouseButton::Left));
    assert!(state.selection_size.field_drag.is_some());
    let rect = state
        .selection_size
        .panel
        .as_ref()
        .unwrap()
        .field_rect(Field::Width)
        .unwrap();
    pointer(&mut state, f64::from(rect.left()) - 8.0, point.1, &mut 0);
    assert_eq!(
        state.selection_size.panel.as_ref().unwrap().fields[0]
            .editor
            .selection_range(),
        Some((0, anchor))
    );
    button(&mut state, MouseButton::Left, false);
    assert!(state.selection_size.field_drag.is_none() && state.selection_size.pressed.is_none());
    assert!(!state.input.mouse_down);
}

/// 【选区尺寸测试】【焦点交接】拖选中 Tab 提交并切换字段后，旧字段移动不能改写新字段选择
#[test]
fn focus_transfer_invalidates_the_previous_field_drag() {
    let mut state = editing();
    let point = center(&state);
    pointer(&mut state, point.0, point.1, &mut 0);
    button(&mut state, MouseButton::Left, true);
    key(&mut state, "Tab", &mut 0);
    pointer(&mut state, point.0 + 80.0, point.1, &mut 0);
    let panel = state.selection_size.panel.as_ref().unwrap();
    assert_eq!(panel.editing(), Some(Field::Height));
    assert_eq!(
        panel.fields[1].editor.selected_text().as_deref(),
        Some("180")
    );
    assert!(state.selection_size.field_drag.is_none());
    assert_eq!(state.selection.zone.unwrap().width(), 120.0);
    button(&mut state, MouseButton::Left, false);
}

/// 【选区尺寸测试】【跨输出拖选】指针进入另一比例输出时，字段继续使用原输出上的全局坐标，不使用新输出的局部零点
#[test]
fn captured_field_drag_uses_global_coordinates_across_outputs() {
    let mut state = editing();
    state.placements.push(crate::types::Placement {
        position: (640, 0),
        size: (640, 480),
        scale: 1.5,
    });
    let point = center(&state);
    pointer(&mut state, point.0, point.1, &mut 0);
    button(&mut state, MouseButton::Left, true);
    super::super::handle_pointer_move(&mut state, 1, 0.0, point.1, &mut 0);
    assert_eq!(state.input.pointer.monitor_idx, 1);
    assert_eq!(state.selection_size.panel.as_ref().unwrap().monitor, 0);
    assert_eq!(
        state.selection_size.panel.as_ref().unwrap().fields[0]
            .editor
            .cursor,
        4
    );
    assert!(state.selection_size.field_drag.is_some());
    button(&mut state, MouseButton::Left, false);
    assert!(!state.input.mouse_down);
}

/// 【选区尺寸测试】【文字改写】拖选中上下键改写文字后，输入法身份更新，但原字段的鼠标选择仍继续
#[test]
fn arrow_replacement_keeps_the_existing_field_mouse_grab() {
    let mut state = editing();
    let point = center(&state);
    pointer(&mut state, point.0, point.1, &mut 0);
    button(&mut state, MouseButton::Left, true);
    let previous = state.selection_size.panel.as_ref().unwrap().fields[0].id;
    key(&mut state, "Up", &mut 0);
    assert_ne!(
        state.selection_size.panel.as_ref().unwrap().fields[0].id,
        previous
    );
    let rect = state
        .selection_size
        .panel
        .as_ref()
        .unwrap()
        .field_rect(Field::Width)
        .unwrap();
    pointer(&mut state, f64::from(rect.left()) - 8.0, point.1, &mut 0);
    let field = &state.selection_size.panel.as_ref().unwrap().fields[0];
    assert_eq!(field.editor.text, "121");
    assert_eq!(field.editor.cursor, 0);
    assert!(field.editor.selection_range().is_none());
    assert!(state.selection_size.field_drag.is_some());
    button(&mut state, MouseButton::Left, false);
}

/// 【选区尺寸测试】【连续多击】双击后上下键改写正文不结束原版三击计时，下一次点击仍选择新正文
#[test]
fn arrow_replacement_preserves_the_triple_click_sequence() {
    let mut state = editing();
    let point = center(&state);
    pointer(&mut state, point.0, point.1, &mut 0);
    for _ in 0..2 {
        for pressed in [true, false] {
            button(&mut state, MouseButton::Left, pressed);
        }
    }
    key(&mut state, "Up", &mut 0);
    for pressed in [true, false] {
        button(&mut state, MouseButton::Left, pressed);
    }
    assert_eq!(
        state.selection_size.panel.as_ref().unwrap().fields[0]
            .editor
            .selected_text()
            .as_deref(),
        Some("121")
    );
}

/// 【选区尺寸测试】【滚轮改写】左键持有期间滚轮立即应用尺寸，拖选继续操作同一字段的新正文
#[test]
fn wheel_replacement_keeps_the_existing_field_mouse_grab() {
    let mut state = editing();
    let point = center(&state);
    pointer(&mut state, point.0, point.1, &mut 0);
    button(&mut state, MouseButton::Left, true);
    let previous = state.selection_size.panel.as_ref().unwrap().fields[0].id;
    super::super::handle_axis_scroll(
        &mut state,
        crate::types::ScrollEvent::pixels(0.0, 1.0),
        &mut 0,
    );
    assert_ne!(
        state.selection_size.panel.as_ref().unwrap().fields[0].id,
        previous
    );
    assert_eq!(state.selection.zone.unwrap().width(), 121.0);
    let rect = state
        .selection_size
        .panel
        .as_ref()
        .unwrap()
        .field_rect(Field::Width)
        .unwrap();
    pointer(&mut state, f64::from(rect.left()) - 8.0, point.1, &mut 0);
    let field = &state.selection_size.panel.as_ref().unwrap().fields[0];
    assert_eq!(field.editor.text, "121");
    assert_eq!(field.editor.cursor, 0);
    assert!(field.editor.selection_range().is_none());
    assert!(state.selection_size.field_drag.is_some());
    button(&mut state, MouseButton::Left, false);
}

/// 【选区尺寸测试】【同步改写】实际尺寸同步保持鼠标拖选，字段关闭并重新编辑后旧拖选失效
#[test]
fn synchronized_replacement_preserves_grab_but_reopening_invalidates_it() {
    let mut state = super::selection_size::selected();
    key(&mut state, "Ctrl+R", &mut 0);
    let point = center(&state);
    pointer(&mut state, point.0, point.1, &mut 0);
    button(&mut state, MouseButton::Left, true);
    let field = &mut state.selection_size.panel.as_mut().unwrap().fields[0];
    let previous = field.id;
    let session = field.session;
    field.sync(320, 640);
    assert_ne!(field.id, previous);
    assert_eq!(field.session, session);
    field.begin();
    assert_eq!(field.session, session);
    let rect = state
        .selection_size
        .panel
        .as_ref()
        .unwrap()
        .field_rect(Field::Width)
        .unwrap();
    pointer(&mut state, f64::from(rect.left()) - 8.0, point.1, &mut 0);
    assert!(state.selection_size.field_drag.is_some());
    let field = &mut state.selection_size.panel.as_mut().unwrap().fields[0];
    assert_eq!(field.editor.cursor, 0);
    field.cancel();
    field.begin();
    assert_ne!(field.session, session);
    pointer(&mut state, point.0, point.1, &mut 0);
    assert!(state.selection_size.field_drag.is_none());
    assert_eq!(
        state.selection_size.panel.as_ref().unwrap().fields[0]
            .editor
            .selected_text()
            .as_deref(),
        Some("320")
    );
    button(&mut state, MouseButton::Left, false);
}
