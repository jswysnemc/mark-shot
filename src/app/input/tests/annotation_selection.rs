//! 【标注删除测试】【应用历史】通过真实动作分派检查删除前选择身份和撤销恢复

use super::*;
use crate::{
    tools::Tool,
    types::{Annotation, AnnotationShape},
};

/// 【标注删除测试】【已选矩形】返回有一个真实标注和可用截图选区的编辑器，无原生窗口
pub(super) fn selected() -> EditorState {
    let mut state = editor();
    state.selection.zone = tiny_skia::Rect::from_xywh(20.0, 20.0, 500.0, 400.0);
    state.selected_tool = Tool::Pick;
    state.annotations.push(Annotation {
        style_scale: 1.0,
        rotation_degrees: 0.0,
        frozen_bounds: None,
        id: 7,
        shape: AnnotationShape::Rectangle {
            radius: 0.0,
            fixed_ratio: false,
            start: (180.0, 130.0),
            end: (280.0, 210.0),
            filled: false,
        },
        color: Color::from_rgba8(30, 40, 50, 255),
        shadow_color: Color::TRANSPARENT,
        stroke_width: 3.0,
        bbox: tiny_skia::Rect::from_xywh(180.0, 130.0, 100.0, 80.0).unwrap(),
    });
    state.select_annotation_index(Some(0));
    state
}

/// 【标注删除测试】【选择恢复】删除及撤销必须恢复原选中标注，不能在记录快照前清除选择身份
#[test]
fn undo_after_delete_restores_the_selected_annotation_identity() {
    let mut state = selected();
    key(&mut state, "Delete", &mut 0);
    assert!(state.annotations.is_empty());
    key(&mut state, "Ctrl+Z", &mut 0);
    assert_eq!(state.annotations.len(), 1);
    assert_eq!(state.annotations[0].id, 7);
    assert_eq!(state.selected_annotation_index(), Some(0));
}

/// 【标注删除测试】【左键归属】红色按钮只由左键按下删除，释放不重复提交，撤销恢复后重做再次删除
#[test]
fn delete_button_handles_left_press_once_and_keeps_right_button_inert() {
    let mut state = selected();
    crate::app::annotation_selection::sync(&mut state, &mut 0);
    let rect = state.annotation_selection.frame.unwrap().1;
    pointer(
        &mut state,
        f64::from(rect.x() + 10.0),
        f64::from(rect.y() + 10.0),
        &mut 0,
    );
    assert_eq!(
        super::super::compute_cursor(&state),
        crate::types::CursorIcon::Pointer
    );
    super::super::handle_pointer_button(&mut state, crate::types::MouseButton::Right, true, &mut 0);
    super::super::handle_pointer_button(
        &mut state,
        crate::types::MouseButton::Right,
        false,
        &mut 0,
    );
    assert_eq!(state.annotations.len(), 1);
    super::super::handle_pointer_button(&mut state, crate::types::MouseButton::Left, true, &mut 0);
    assert!(state.annotations.is_empty());
    assert!(state.annotation_selection.frame.is_none());
    super::super::handle_pointer_button(
        &mut state,
        crate::types::MouseButton::Right,
        false,
        &mut 0,
    );
    super::super::handle_pointer_button(&mut state, crate::types::MouseButton::Left, false, &mut 0);
    assert_eq!(state.undo_stack.len(), 1);
    key(&mut state, "Ctrl+Z", &mut 0);
    assert_eq!(state.selected_annotation_index(), Some(0));
    key(&mut state, "Ctrl+Y", &mut 0);
    assert!(state.annotations.is_empty());
}

/// 【标注选框测试】【缩放几何】按可见右边中点拖动时正文端点跟随指针，描边外伸不能改变缩放参照边界
#[test]
fn rectangle_resize_uses_visible_shape_bounds_without_shadow_padding() {
    let mut state = selected();
    state.annotations[0].update_bbox();
    pointer(&mut state, 280.0, 170.0, &mut 0);
    super::super::handle_pointer_button(&mut state, crate::types::MouseButton::Left, true, &mut 0);
    pointer(&mut state, 300.0, 170.0, &mut 0);
    super::super::handle_pointer_button(&mut state, crate::types::MouseButton::Left, false, &mut 0);
    assert!(matches!(
        state.annotations[0].shape,
        AnnotationShape::Rectangle {
            start: (180.0, 130.0),
            end: (300.0, 210.0),
            ..
        }
    ));
    assert_eq!(state.undo_stack.len(), 1);
}

/// 【标注删除测试】【整项职责】线条当前选中控制点时，红色按钮仍删除整条标注，撤销恢复完整骨架
#[test]
fn red_delete_button_removes_the_whole_line_even_with_selected_control() {
    let mut state = selected();
    let mut line =
        crate::features::line::LineSkeleton::new((180.0, 130.0), (280.0, 210.0)).unwrap();
    line.controls.push((220.0, 170.0));
    state.annotations[0].shape = AnnotationShape::Line {
        line,
        style: Default::default(),
    };
    state.annotations[0].update_bbox();
    state.line_session.selected_point = Some((7, 2));
    crate::app::annotation_selection::sync(&mut state, &mut 0);
    let rect = state.annotation_selection.frame.unwrap().1;
    pointer(
        &mut state,
        f64::from(rect.x() + 10.0),
        f64::from(rect.y() + 10.0),
        &mut 0,
    );
    super::super::handle_pointer_button(&mut state, crate::types::MouseButton::Left, true, &mut 0);
    super::super::handle_pointer_button(&mut state, crate::types::MouseButton::Left, false, &mut 0);
    assert!(state.annotations.is_empty());
    key(&mut state, "Ctrl+Z", &mut 0);
    assert_eq!(
        state.annotations[0].shape.line().unwrap().controls,
        [(220.0, 170.0)]
    );
    assert_eq!(state.selected_annotation_index(), Some(0));
}

/// 【标注选框测试】【外部缩放点】粗笔迹的可见控制点在正文命中范围外，仍应优先操作当前标注而不是清除选择
#[test]
fn wide_pen_control_outside_initial_hit_bounds_starts_resize() {
    let mut state = selected();
    state.annotations[0].shape = AnnotationShape::Pen {
        points: vec![(180.0, 130.0), (280.0, 210.0)],
    };
    state.annotations[0].stroke_width = 40.0;
    state.annotations[0].update_bbox();
    assert!(!state.annotations[0].initial_hit_test((131.0, 170.0)));
    pointer(&mut state, 131.0, 170.0, &mut 0);
    super::super::handle_pointer_button(&mut state, crate::types::MouseButton::Left, true, &mut 0);
    assert_eq!(state.selected_annotation_index(), Some(0));
    assert_eq!(
        state.ann_drag.as_ref().unwrap().handle,
        crate::types::SelectionHandle::Left
    );
    super::super::handle_pointer_button(&mut state, crate::types::MouseButton::Left, false, &mut 0);
    assert_eq!(state.selected_annotation_index(), Some(0));
}

/// 【标注删除测试】【悬停缓存】无参数，进入退出重新绘制且不扩大命中，稳定指针不失效缓存或改动标注，无返回值
#[test]
fn delete_hover_refreshes_cache_without_changing_hit_target_or_document() {
    let mut state = selected();
    crate::app::annotation_selection::sync(&mut state, &mut 0);
    let rect = state.annotation_selection.frame.unwrap().1;
    let scale = state.placements[0].scale;
    let original = state.annotations.clone();
    pointer(
        &mut state,
        f64::from(rect.x() + 10.0),
        f64::from(rect.y() + 10.0),
        &mut 0,
    );
    let mut dirty = 0;
    crate::app::annotation_selection::sync(&mut state, &mut dirty);
    assert_ne!(dirty, 0);
    assert!(
        state
            .annotation_selection
            .cache
            .as_ref()
            .unwrap()
            .matches(rect, scale, true)
    );
    dirty = 0;
    crate::app::annotation_selection::sync(&mut state, &mut dirty);
    assert_eq!(dirty, 0);
    pointer(
        &mut state,
        f64::from(rect.right() + 1.0),
        f64::from(rect.y() + 10.0),
        &mut 0,
    );
    assert!(!crate::app::annotation_selection::hovered(&state));
    crate::app::annotation_selection::sync(&mut state, &mut dirty);
    assert_ne!(dirty, 0);
    assert!(
        state
            .annotation_selection
            .cache
            .as_ref()
            .unwrap()
            .matches(rect, scale, false)
    );
    assert_eq!(state.annotations, original);
    assert!(state.undo_stack.is_empty());
}
