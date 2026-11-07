//! 【普通标注旋转测试】【输入入口】核对原版旋转控制点和实际拖动，不通过替代模型模拟操作

use super::*;

/// 【普通标注旋转测试】【原版控制点】100×80矩形上边外26逻辑像素应命中旋转控制点，无参数、无返回值
#[test]
fn ordinary_rectangle_exposes_the_qt_rotation_handle() {
    let state = super::annotation_selection::selected();
    let annotation = &state.annotations[0];
    assert_eq!(
        crate::editor::annotation_drag::handle_hit_test_for_annotation(annotation, (230.0, 104.0)),
        crate::types::SelectionHandle::Rotate
    );
}

/// 【普通标注旋转测试】【实际选框】旋转控制点应由实际共用组件绘制，不能仅增加不可见命中区域，无参数、无返回值
#[test]
fn ordinary_rectangle_draws_the_qt_rotation_handle() {
    let mut state = super::annotation_selection::selected();
    let mut canvas = Pixmap::new(640, 480).unwrap();
    crate::renderer::draw_annotation(
        &mut crate::renderer::Painter::new(&mut canvas),
        &state.annotations[0],
        true,
        &mut state.text.editors,
        &state.filters.images,
        None,
    );
    let pixel = canvas.pixel(230, 104).unwrap().demultiply();
    assert_eq!(
        (pixel.red(), pixel.green(), pixel.blue(), pixel.alpha()),
        (251, 146, 60, 255)
    );
}

/// 【普通标注旋转测试】【实际拖动】在 state 中以左键从 start 拖到 end 并释放，不直接修改标注角度，无返回值
fn rotate(state: &mut EditorState, start: (f64, f64), end: (f64, f64)) {
    pointer(state, start.0, start.1, &mut 0);
    super::super::handle_pointer_button(state, crate::types::MouseButton::Left, true, &mut 0);
    assert_eq!(
        state.ann_drag.as_ref().unwrap().handle,
        crate::types::SelectionHandle::Rotate
    );
    pointer(state, end.0, end.1, &mut 0);
    super::super::handle_pointer_button(state, crate::types::MouseButton::Left, false, &mut 0);
}

/// 【普通标注旋转测试】【完整历史】90度旋转保留原始矩形和身份，撤销恢复选择并重做角度，无参数、无返回值
#[test]
fn rotation_keeps_original_geometry_and_restores_selected_history() {
    let mut state = super::annotation_selection::selected();
    let before = state.annotations[0].clone();
    rotate(&mut state, (230.0, 104.0), (296.0, 170.0));
    assert_eq!(state.annotations[0].rotation_degrees, 90.0);
    assert_eq!(state.annotations[0].shape, before.shape);
    assert_eq!(state.annotations[0].bbox, before.bbox);
    assert_eq!(state.selected_annotation_index(), Some(0));
    assert_eq!(state.undo_stack.len(), 1);
    key(&mut state, "Ctrl+Z", &mut 0);
    assert_eq!(state.annotations[0], before);
    assert_eq!(state.selected_annotation_index(), Some(0));
    key(&mut state, "Ctrl+Y", &mut 0);
    assert_eq!(state.annotations[0].rotation_degrees, 90.0);
    assert_eq!(state.selected_annotation_index(), Some(0));
}

/// 【普通标注旋转测试】【修饰键吸附】按住Shift的约60度手势应吸附到45度，不以鼠标增量角度累计，无参数、无返回值
#[test]
fn shift_rotation_snaps_to_the_absolute_qt_45_degree_grid() {
    let mut state = super::annotation_selection::selected();
    state.input.shift = true;
    rotate(&mut state, (230.0, 104.0), (287.157_68, 137.0));
    assert_eq!(state.annotations[0].rotation_degrees, 45.0);
}

/// 【普通标注旋转测试】【右键归属】右键不开始旋转或写入历史，保留已有角度和正文，无参数、无返回值
#[test]
fn right_button_does_not_begin_an_ordinary_rotation() {
    let mut state = super::annotation_selection::selected();
    let before = state.annotations[0].clone();
    pointer(&mut state, 230.0, 104.0, &mut 0);
    super::super::handle_pointer_button(&mut state, crate::types::MouseButton::Right, true, &mut 0);
    pointer(&mut state, 296.0, 170.0, &mut 0);
    super::super::handle_pointer_button(
        &mut state,
        crate::types::MouseButton::Right,
        false,
        &mut 0,
    );
    assert_eq!(state.annotations[0], before);
    assert!(state.ann_drag.is_none());
    assert!(state.undo_stack.is_empty());
}

/// 【普通标注旋转测试】【草稿取消】左键旋转途中按下右键恢复原始角度与选择，不把已取消手势写入历史，无参数、无返回值
#[test]
fn right_press_cancels_pending_rotation_without_a_history_entry() {
    let mut state = super::annotation_selection::selected();
    let before = state.annotations[0].clone();
    pointer(&mut state, 230.0, 104.0, &mut 0);
    super::super::handle_pointer_button(&mut state, crate::types::MouseButton::Left, true, &mut 0);
    pointer(&mut state, 296.0, 170.0, &mut 0);
    assert_eq!(state.pending.as_ref().unwrap().rotation_degrees, 90.0);
    super::super::handle_pointer_button(&mut state, crate::types::MouseButton::Right, true, &mut 0);
    super::super::handle_pointer_button(
        &mut state,
        crate::types::MouseButton::Right,
        false,
        &mut 0,
    );
    super::super::handle_pointer_button(&mut state, crate::types::MouseButton::Left, false, &mut 0);
    assert_eq!(state.annotations[0], before);
    assert_eq!(state.selected_annotation_index(), Some(0));
    assert!(state.pending.is_none());
    assert!(state.undo_stack.is_empty());
}

/// 【普通标注旋转测试】【反向缩放】90度矩形的可见右边控制点应沿本地横向缩放，并保持原角度，无参数、无返回值
#[test]
fn rotated_resize_maps_pointer_back_to_unrotated_shape_bounds() {
    let mut state = super::annotation_selection::selected();
    state.annotations[0].rotation_degrees = 90.0;
    pointer(&mut state, 230.0, 220.0, &mut 0);
    super::super::handle_pointer_button(&mut state, crate::types::MouseButton::Left, true, &mut 0);
    assert_eq!(
        state.ann_drag.as_ref().unwrap().handle,
        crate::types::SelectionHandle::Right
    );
    pointer(&mut state, 230.0, 240.0, &mut 0);
    super::super::handle_pointer_button(&mut state, crate::types::MouseButton::Left, false, &mut 0);
    assert_eq!(state.annotations[0].rotation_degrees, 90.0);
    assert!(matches!(
        state.annotations[0].shape,
        crate::types::AnnotationShape::Rectangle {
            start: (180.0, 130.0),
            end: (300.0, 210.0),
            ..
        }
    ));
}

/// 【普通标注旋转测试】【文字重新编辑】经真实文字输入提交保留角度及身份，编辑草稿不改持久正文，历史恢复同一旋转姿态，无参数、无返回值
#[test]
fn editing_rotated_text_preserves_pose_identity_and_global_history() {
    let mut state = editor();
    super::text::add_text(&mut state);
    state.annotations[0].rotation_degrees = 45.0;
    state.selected_tool = crate::tools::Tool::Pick;
    let before = state.annotations[0].clone();
    assert!(crate::editor::text_session::begin_existing(
        &mut state, before.id, &mut 0
    ));
    assert_eq!(
        state
            .text_session
            .as_ref()
            .unwrap()
            .working
            .rotation_degrees,
        45.0
    );
    key(&mut state, "Ctrl+A", &mut 0);
    super::super::handle_key(&mut state, None, Some("edited  ".into()), &mut 0);
    assert_eq!(state.annotations[0], before);
    key(&mut state, "Ctrl+Return", &mut 0);
    let after = state.annotations[0].clone();
    assert_eq!(after.id, before.id);
    assert_eq!(after.rotation_degrees, 45.0);
    assert!(
        matches!(&after.shape, crate::types::AnnotationShape::Text { content, .. } if content == "edited")
    );
    assert_eq!(state.undo_stack.len(), 1);
    key(&mut state, "Ctrl+Z", &mut 0);
    assert_eq!(state.annotations[0], before);
    key(&mut state, "Ctrl+Y", &mut 0);
    assert_eq!(state.annotations[0], after);
}

/// 【普通标注旋转测试】【单行边缩放】真实右边中点扩宽改变换行参数，原版可见选框仍按自然字形收紧，角度和字号保持不变，无参数、无返回值
#[test]
fn rotated_single_line_text_widening_keeps_qt_visible_bounds() {
    let mut state = editor();
    super::text::add_text(&mut state);
    state.annotations[0].rotation_degrees = 90.0;
    state.selected_tool = crate::tools::Tool::Pick;
    let before = state.annotations[0].clone();
    let center = before.rotation_center();
    let handle = before.to_visual((before.bbox.right(), center.1));
    pointer(&mut state, f64::from(handle.0), f64::from(handle.1), &mut 0);
    super::super::handle_pointer_button(&mut state, crate::types::MouseButton::Left, true, &mut 0);
    assert_eq!(
        state.ann_drag.as_ref().unwrap().handle,
        crate::types::SelectionHandle::Right
    );
    pointer(
        &mut state,
        f64::from(handle.0),
        f64::from(handle.1) + 24.0,
        &mut 0,
    );
    super::super::handle_pointer_button(&mut state, crate::types::MouseButton::Left, false, &mut 0);
    assert_eq!(state.annotations[0].bbox, before.bbox);
    assert!(
        matches!(&state.annotations[0].shape, crate::types::AnnotationShape::Text { frame, .. }
        if frame.width == Some(before.bbox.width() + 24.0))
    );
    assert_eq!(state.annotations[0].rotation_degrees, 90.0);
    let after = state.annotations[0].clone();
    assert_eq!(state.undo_stack.len(), 1);
    key(&mut state, "Ctrl+Z", &mut 0);
    assert_eq!(state.annotations[0], before);
    key(&mut state, "Ctrl+Y", &mut 0);
    assert_eq!(state.annotations[0], after);
    let crate::types::AnnotationShape::Text { font_size, .. } = state.annotations[0].shape else {
        unreachable!()
    };
    let crate::types::AnnotationShape::Text {
        font_size: old_size,
        ..
    } = before.shape
    else {
        unreachable!()
    };
    assert_eq!(font_size, old_size);
}
