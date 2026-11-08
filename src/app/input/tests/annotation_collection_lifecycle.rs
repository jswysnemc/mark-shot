//! 【标注多选测试】【手势衔接】真实输入覆盖焦点中断、同工具重选、拖动期间历史与旋转后单项线条编辑

use super::*;
use crate::{
    features::annotation_collection::geometry,
    types::{AnnotationShape, MouseButton, SelectionHandle},
};

/// 【标注多选测试】【左键事件】按pressed向state发送真实左键状态，无返回值
fn left(state: &mut EditorState, pressed: bool) {
    super::super::handle_pointer_button(state, MouseButton::Left, pressed, &mut 0);
}

/// 【标注多选测试】【群组移动】在state开始共同移动并更新一次，保留左键按下状态，无返回值
fn moving(state: &mut EditorState) {
    pointer(state, 300.0, 220.0, &mut 0);
    left(state, true);
    pointer(state, 330.0, 240.0, &mut 0);
    assert!(state.collection.drag.is_some());
}

/// 【标注多选测试】【焦点移出】失去实际键盘焦点结束群组手势，保留已更新几何，恢复焦点后旧释放不能二次提交
#[test]
fn focus_loss_finishes_group_drag_and_discards_an_uncommitted_marquee() {
    let mut state = super::annotation_collection::group();
    state.input.keyboard_focus = Some(0);
    moving(&mut state);
    let after = state.annotations.clone();
    crate::app::event_loop::handle_event(
        &mut state,
        crate::types::OverlayEvent::FocusLost { monitor_idx: 0 },
        &mut 0,
    );
    assert!(state.collection.drag.is_none());
    assert!(!state.input.mouse_down);
    assert_eq!(state.annotations, after);
    assert_eq!(state.input.keyboard_focus, None);
    left(&mut state, false);
    assert_eq!(state.undo_stack.len(), 1);
    pointer(&mut state, 60.0, 60.0, &mut 0);
    left(&mut state, true);
    pointer(&mut state, 400.0, 350.0, &mut 0);
    crate::app::event_loop::handle_event(
        &mut state,
        crate::types::OverlayEvent::FocusLost { monitor_idx: 0 },
        &mut 0,
    );
    left(&mut state, false);
    assert!(state.collection.marquee.is_none());
    assert!(state.selected_annotation_ids().is_empty());
    assert_eq!(state.annotations, after);
    assert_eq!(state.undo_stack.len(), 1);
}

/// 【标注多选测试】【选择重选】再次按选择快捷键结束手势并保留群组，切换绘图工具清除群组，不额外记录历史
#[test]
fn selecting_pick_again_keeps_the_group_and_finishes_current_drag() {
    let mut state = super::annotation_collection::group();
    moving(&mut state);
    let after = state.annotations.clone();
    key(&mut state, "V", &mut 0);
    assert!(state.collection.drag.is_none());
    assert_eq!(state.selected_annotation_ids(), &[17, 7]);
    left(&mut state, false);
    assert_eq!(state.annotations, after);
    assert_eq!(state.undo_stack.len(), 1);
    key(&mut state, "R", &mut 0);
    assert!(state.selected_annotation_ids().is_empty());
    assert_eq!(state.annotations, after);
}

/// 【标注多选测试】【未释放历史】拖动时撤销结束旧手势，继续移动不修改已恢复几何，未释放时重做及最终释放保持同一结果
#[test]
fn undo_and_redo_during_a_held_drag_stop_the_old_gesture() {
    let mut state = super::annotation_collection::group();
    let before = state.annotations.clone();
    moving(&mut state);
    let after = state.annotations.clone();
    key(&mut state, "Ctrl+Z", &mut 0);
    assert!(state.collection.drag.is_none());
    assert_eq!(state.annotations, before);
    pointer(&mut state, 400.0, 280.0, &mut 0);
    assert_eq!(state.annotations, before);
    key(&mut state, "Ctrl+Y", &mut 0);
    assert_eq!(state.annotations, after);
    left(&mut state, false);
    assert_eq!(state.annotations, after);
    assert_eq!(state.undo_stack.len(), 1);
    assert_eq!(state.selected_annotation_ids(), &[17, 7]);
}

/// 【标注多选测试】【旋转线条构造】返回已经通过真实群组手势旋转九十度的线条与矩形，退出选择后无选中成员
fn rotated_line_group() -> EditorState {
    let mut state = super::annotation_collection::group();
    state.annotations[0].shape = AnnotationShape::Line {
        line: crate::features::line::LineSkeleton::new((180.0, 130.0), (280.0, 210.0)).unwrap(),
        style: Default::default(),
    };
    state.annotations[0].update_bbox();
    let bounds = state.collection_bounds().unwrap();
    let center = crate::features::annotation_selection::chrome::center(bounds);
    let start = geometry::rotation_handle(bounds).1;
    let end = crate::features::annotation_rotation::geometry::rotated(start, center, 90.0);
    pointer(&mut state, f64::from(start.0), f64::from(start.1), &mut 0);
    left(&mut state, true);
    pointer(&mut state, f64::from(end.0), f64::from(end.1), &mut 0);
    left(&mut state, false);
    assert_eq!(state.annotations[0].rotation_degrees, 90.0);
    pointer(&mut state, 60.0, 60.0, &mut 0);
    left(&mut state, true);
    left(&mut state, false);
    assert!(state.selected_annotation_ids().is_empty());
    state
}

/// 【标注多选测试】【线条首次重选】群组旋转后首次按住显示端点直接开始端点编辑，保留角度与编号，撤销重做恢复同一单项选择
#[test]
fn group_rotated_line_can_be_reselected_by_its_visual_endpoint() {
    let mut state = rotated_line_group();
    let before = state.annotations[0].clone();
    let point = before.to_visual(before.shape.line().unwrap().start);
    pointer(&mut state, f64::from(point.0), f64::from(point.1), &mut 0);
    left(&mut state, true);
    assert_eq!(
        state.ann_drag.as_ref().unwrap().handle,
        SelectionHandle::LinePoint(0)
    );
    pointer(
        &mut state,
        f64::from(point.0) + 24.0,
        f64::from(point.1) - 14.0,
        &mut 0,
    );
    left(&mut state, false);
    let after = state.annotations[0].clone();
    assert_ne!(after.shape, before.shape);
    assert_eq!(after.rotation_degrees, 90.0);
    assert_eq!(after.id, before.id);
    assert_eq!(state.selected_annotation_ids(), &[7]);
    assert_eq!(state.undo_stack.len(), 2);
    key(&mut state, "Ctrl+Z", &mut 0);
    assert_eq!(state.annotations[0], before);
    assert_eq!(state.selected_annotation_ids(), &[7]);
    key(&mut state, "Ctrl+Y", &mut 0);
    assert_eq!(state.annotations[0], after);
}

/// 【标注多选测试】【线条双击】共同旋转后按可见线身双击插入本地控制点，删除优先移除控制点，不删除整条线
#[test]
fn group_rotated_line_double_click_inserts_a_local_control() {
    let mut state = rotated_line_group();
    let annotation = &state.annotations[0];
    let line = annotation.shape.line().unwrap();
    let point = annotation.to_visual((
        (line.start.0 + line.end.0) / 2.0,
        (line.start.1 + line.end.1) / 2.0,
    ));
    for _ in 0..2 {
        pointer(&mut state, f64::from(point.0), f64::from(point.1), &mut 0);
        left(&mut state, true);
        left(&mut state, false);
    }
    assert_eq!(state.annotations[0].shape.line().unwrap().controls.len(), 1);
    assert_eq!(state.annotations[0].rotation_degrees, 90.0);
    key(&mut state, "Delete", &mut 0);
    assert_eq!(state.annotations.len(), 2);
    assert!(
        state.annotations[0]
            .shape
            .line()
            .unwrap()
            .controls
            .is_empty()
    );
    assert_eq!(state.selected_annotation_ids(), &[7]);
}
