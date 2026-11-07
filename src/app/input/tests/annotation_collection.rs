//! 【标注多选测试】【真实输入】按生产框选与共同控制框验证图层、编号、历史和批量属性

use super::*;
use crate::{
    features::annotation_collection::{geometry, selection::Selection},
    types::{AnnotationShape, MouseButton, SelectionHandle},
};

/// 【标注多选测试】【群组构造】返回两项已选矩形的真实编辑器，选择编号顺序与图层顺序不同，无参数
pub(super) fn group() -> EditorState {
    let mut state = super::annotation_selection::selected();
    let mut second = state.annotations[0].clone();
    second.id = 17;
    second.shape = AnnotationShape::Rectangle {
        start: (340.0, 250.0),
        end: (420.0, 310.0),
        filled: false,
    };
    second.color = Color::from_rgba8(140, 70, 45, 255);
    second.update_bbox();
    state.annotations.push(second);
    state.select_annotation_ids([17, 7]);
    state.ui_hidden = true;
    state
}

/// 【标注多选测试】【左键输入】按pressed给state发送实际按钮，返回值为空
fn left(state: &mut EditorState, pressed: bool) {
    super::super::handle_pointer_button(state, MouseButton::Left, pressed, &mut 0);
}

/// 【标注多选测试】【有序身份】无效编号和重复值不会进入选择，单项投影只在有效成员恰好为一时出现
#[test]
fn ids_keep_requested_order_and_remove_invalid_duplicates() {
    let mut selection = Selection::default();
    assert!(selection.replace([7, 8, 7, 9, 8], |id| id != 9));
    assert_eq!(selection.ids(), &[7, 8]);
    assert_eq!(selection.single(), None);
    assert!(!selection.replace([7, 8, 7], |_| true));
    let mut state = group();
    assert_eq!(state.selected_annotation_ids(), &[17, 7]);
    assert_eq!(state.selected_annotation_index(), None);
    state.select_annotation_ids([99, 7, 7]);
    assert_eq!(state.selected_annotation_index(), Some(0));
}

/// 【标注多选测试】【相交框选】从空白开始时立即清除旧组，松开选择相交对象，不要求完整包含，也不写历史
#[test]
fn blank_drag_selects_intersections_in_original_layer_order() {
    let mut state = group();
    pointer(&mut state, 60.0, 60.0, &mut 0);
    left(&mut state, true);
    assert!(state.selected_annotation_ids().is_empty());
    assert!(state.collection.marquee.is_some());
    pointer(&mut state, 350.0, 260.0, &mut 0);
    left(&mut state, false);
    assert_eq!(state.selected_annotation_ids(), &[7, 17]);
    assert!(state.collection.marquee.is_none());
    assert!(state.undo_stack.is_empty());
    let canvas = tiny_skia::Rect::from_xywh(0.0, 0.0, 640.0, 480.0).unwrap();
    let tiny = tiny_skia::Rect::from_xywh(180.0, 130.0, 1.25, 2.0).unwrap();
    assert!(geometry::in_rect(&state.annotations, tiny, canvas, 1.5).is_empty());
    assert_eq!(
        geometry::in_rect(&state.annotations, tiny, canvas, 2.0),
        [7]
    );
}

/// 【标注多选测试】【共同移动】空隙位于联合框内时先命中整组，保持原始图层与选择顺序，一次拖动只形成一次历史
#[test]
fn group_frame_has_priority_and_move_keeps_layer_order() {
    let mut state = group();
    let before = state.annotations.clone();
    pointer(&mut state, 300.0, 220.0, &mut 0);
    left(&mut state, true);
    assert_eq!(
        state.collection.drag.as_ref().unwrap().handle,
        SelectionHandle::Move
    );
    assert!(state.undo_stack.is_empty());
    pointer(&mut state, 330.0, 230.0, &mut 0);
    pointer(&mut state, 350.0, 260.0, &mut 0);
    left(&mut state, false);
    assert_eq!(
        state
            .annotations
            .iter()
            .map(|annotation| annotation.id)
            .collect::<Vec<_>>(),
        [7, 17]
    );
    assert_eq!(state.selected_annotation_ids(), &[17, 7]);
    assert!(matches!(
        state.annotations[0].shape,
        AnnotationShape::Rectangle {
            start: (230.0, 170.0),
            end: (330.0, 250.0),
            ..
        }
    ));
    assert_eq!(state.undo_stack.len(), 1);
    let after = state.annotations.clone();
    key(&mut state, "Ctrl+Z", &mut 0);
    assert_eq!(state.annotations, before);
    assert_eq!(state.selected_annotation_ids(), &[17, 7]);
    key(&mut state, "Ctrl+Y", &mut 0);
    assert_eq!(state.annotations, after);
    assert_eq!(state.selected_annotation_ids(), &[17, 7]);
}

/// 【标注多选测试】【首次更新历史】无移动点击不保存历史，实际收到零位移更新时仍按生产行为捕获一次
#[test]
fn group_history_distinguishes_click_from_first_zero_update() {
    for update in [false, true] {
        let mut state = group();
        pointer(&mut state, 300.0, 220.0, &mut 0);
        left(&mut state, true);
        if update {
            pointer(&mut state, 300.0, 220.0, &mut 0);
        }
        left(&mut state, false);
        assert_eq!(state.undo_stack.len(), usize::from(update));
    }
}

/// 【标注多选测试】【有界移动】以联合可见边界约束整组位移，成员间距保持不变，矩形边可以贴原图右下边
#[test]
fn moving_group_to_canvas_edge_preserves_member_spacing() {
    let mut state = group();
    pointer(&mut state, 300.0, 220.0, &mut 0);
    left(&mut state, true);
    pointer(&mut state, 900.0, 900.0, &mut 0);
    left(&mut state, false);
    assert_eq!(
        state.collection_bounds().unwrap(),
        tiny_skia::Rect::from_xywh(400.0, 300.0, 240.0, 180.0).unwrap()
    );
    assert!(matches!(
        state.annotations[1].shape,
        AnnotationShape::Rectangle {
            start: (560.0, 420.0),
            end: (640.0, 480.0),
            ..
        }
    ));
}

/// 【标注多选测试】【旋转增量】Shift吸附本次共同增量，成员已有角度分别保留，修饰键变化立即重算而不新增历史
#[test]
fn group_shift_snaps_delta_instead_of_each_absolute_angle() {
    let mut state = group();
    state.annotations[0].rotation_degrees = 13.0;
    state.annotations[1].rotation_degrees = -21.0;
    let bounds = state.collection_bounds().unwrap();
    let center = crate::features::annotation_selection::chrome::center(bounds);
    let handle = geometry::rotation_handle(bounds).1;
    let end = crate::features::annotation_rotation::geometry::rotated(handle, center, 61.0);
    pointer(&mut state, f64::from(handle.0), f64::from(handle.1), &mut 0);
    left(&mut state, true);
    pointer(&mut state, f64::from(end.0), f64::from(end.1), &mut 0);
    super::super::handle_modifiers(&mut state, false, true, &mut 0);
    assert_eq!(state.annotations[0].rotation_degrees, 58.0);
    assert_eq!(state.annotations[1].rotation_degrees, 24.0);
    left(&mut state, false);
    assert_eq!(state.undo_stack.len(), 1);
}

/// 【标注多选测试】【右键结束】右键结束共同移动并保留当前结果，后续左键释放不重复提交，撤销仍恢复整个组
#[test]
fn right_press_ends_group_gesture_and_keeps_updated_geometry() {
    let mut state = group();
    let before = state.annotations.clone();
    pointer(&mut state, 300.0, 220.0, &mut 0);
    left(&mut state, true);
    pointer(&mut state, 350.0, 260.0, &mut 0);
    let after = state.annotations.clone();
    super::super::handle_pointer_button(&mut state, MouseButton::Right, true, &mut 0);
    left(&mut state, false);
    assert!(state.collection.drag.is_none());
    assert_eq!(state.annotations, after);
    assert_eq!(state.undo_stack.len(), 1);
    key(&mut state, "Ctrl+Z", &mut 0);
    assert_eq!(state.annotations, before);
}

/// 【标注多选测试】【组删除按钮】红色按钮删除全部选中成员，一次历史恢复编号顺序与原始图层
#[test]
fn group_delete_button_removes_all_selected_members_once() {
    let mut state = group();
    state.ui_hidden = false;
    let before = state.annotations.clone();
    crate::app::annotation_selection::sync(&mut state, &mut 0);
    let button = state.annotation_selection.frame.unwrap().1;
    assert_eq!((button.x(), button.y()), (428.0, 102.0));
    pointer(
        &mut state,
        f64::from(button.x() + 10.0),
        f64::from(button.y() + 10.0),
        &mut 0,
    );
    left(&mut state, true);
    left(&mut state, false);
    assert!(state.annotations.is_empty());
    assert_eq!(state.undo_stack.len(), 1);
    key(&mut state, "Ctrl+Z", &mut 0);
    assert_eq!(state.annotations, before);
    assert_eq!(state.selected_annotation_ids(), &[17, 7]);
}

/// 【标注多选测试】【批量透明度】通过实际属性滑条改变透明度，成员RGB各自保留，默认工具颜色不改变，选择顺序决定初始值
#[test]
fn native_group_opacity_slider_preserves_each_members_rgb() {
    let mut state = group();
    state.ui_hidden = false;
    super::super::refresh_panels(&mut state, &mut 0);
    assert_eq!(state.settings_panel.widgets.len(), 3);
    let layout = state.settings_panel.text_layout.unwrap();
    assert!(layout.controls[3..].iter().all(Option::is_none));
    let cell = state.settings_panel.widget_rect(1).unwrap();
    let default_color = state.tool_settings.color;
    let start = crate::ui::text_properties::layout::TextLayout::slider_handle(cell, 100, (0, 100));
    pointer(
        &mut state,
        f64::from(start.x() + 7.0),
        f64::from(start.y() + 6.0),
        &mut 0,
    );
    left(&mut state, true);
    pointer(
        &mut state,
        f64::from(cell.x() + (cell.width() - 14.0) * 0.5 + 7.0),
        f64::from(start.y() + 6.0),
        &mut 0,
    );
    left(&mut state, false);
    for (annotation, rgb) in state.annotations.iter().zip([(30, 40, 50), (140, 70, 45)]) {
        let color = annotation.color.to_color_u8();
        assert_eq!((color.red(), color.green(), color.blue()), rgb);
        assert_eq!(color.alpha(), 128);
    }
    assert_eq!(state.tool_settings.color, default_color);
    assert_eq!(state.undo_stack.len(), 1);
}
