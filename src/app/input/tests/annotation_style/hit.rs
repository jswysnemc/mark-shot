//! 【标注命中测试】【首次输入】真实选择输入按窗口容差命中，不借助尚未显示的控制点

use super::*;
use crate::{app::input::handle_axis_scroll, types::ScrollEvent};

/// 【标注命中测试】【直线场景】按wheel返回已有水平线且尚未选中的导航编辑器，width为六个原图像素
fn line_editor(wheel: i32) -> EditorState {
    let mut state = image_editor((1280, 960));
    let mut dirty = 0;
    select_tool(&mut state, Tool::Line, &mut dirty);
    state.tool_settings.set_stroke(StrokeGroup::Line, 6.0);
    pointer(&mut state, 220.0, 240.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    pointer(&mut state, 420.0, 240.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    select_tool(&mut state, Tool::Pick, &mut dirty);
    pointer(&mut state, 320.0, 240.0, &mut dirty);
    handle_axis_scroll(&mut state, ScrollEvent::wheel(wheel), &mut dirty);
    assert!(state.selected_annotation_ids().is_empty());
    state
}

/// 【标注命中测试】【窗口容差】按Qt生产annotationAt的八窗口像素容差首次点击直线，放大与缩小不能扩大或缩小容差
#[test]
fn initial_line_selection_keeps_eight_window_pixels_beyond_projected_half_width() {
    for wheel in [-1440, 1440] {
        for (outside, expected) in [(7.0, true), (9.0, false)] {
            let mut state = line_editor(wheel);
            let view = state.canvas_transform();
            let half = f64::from(state.annotations[0].stroke_width) * view.scale / 2.0;
            let before = state.annotations.clone();
            let mut dirty = 0;
            pointer(&mut state, 320.0, 240.0 + half + outside, &mut dirty);
            handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
            assert_eq!(
                state.selected_annotation_index().is_some(),
                expected,
                "wheel={wheel} outside={outside} view={view:?} canvas={:?} width={} raw={}",
                state.canvas_point(),
                state.annotations[0].stroke_width,
                state.annotations[0].image_stroke_width()
            );
            handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
            assert_eq!(state.annotations, before);
            assert_eq!(state.undo_stack.len(), 1);
        }
    }
}

/// 【标注命中测试】【放大拒绝】原图笔宽不变时放大视图仍拒绝笔迹之外九个窗口像素的首次点击，不把旧画布容差一起放大
#[test]
fn zoomed_in_line_rejects_first_click_nine_window_pixels_outside_the_stroke() {
    let mut state = line_editor(1440);
    let view = state.canvas_transform();
    let half = f64::from(state.annotations[0].stroke_width) * view.scale / 2.0;
    let mut dirty = 0;
    pointer(&mut state, 320.0, 240.0 + half + 9.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    assert_eq!(state.selected_annotation_index(), None);
}

/// 【标注命中测试】【矩形场景】按wheel返回未选中的矩形，几何经真实工具输入创建，宽高足以区分窗口容差
fn rectangle_editor(wheel: i32) -> EditorState {
    let mut state = image_editor((1280, 960));
    let mut dirty = 0;
    select_tool(&mut state, Tool::Rectangle, &mut dirty);
    pointer(&mut state, 280.0, 220.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    pointer(&mut state, 360.0, 260.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    select_tool(&mut state, Tool::Pick, &mut dirty);
    pointer(&mut state, 320.0, 240.0, &mut dirty);
    handle_axis_scroll(&mut state, ScrollEvent::wheel(wheel), &mut dirty);
    state
}

/// 【标注命中测试】【矩形边界】真实首次选择只接受矩形边缘之外八个窗口像素，笔宽和导航倍率不额外扩容
#[test]
fn initial_rectangle_selection_keeps_eight_window_pixels_beyond_geometry() {
    for wheel in [-1440, 1440] {
        for (outside, expected) in [(7.0, true), (9.0, false)] {
            let mut state = rectangle_editor(wheel);
            let mut dirty = 0;
            let view = state.canvas_transform();
            let edge = view.to_screen((360.0, 240.0));
            let before = state.annotations.clone();
            pointer(&mut state, edge.0 + outside, edge.1, &mut dirty);
            handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
            assert_eq!(
                state.selected_annotation_index().is_some(),
                expected,
                "wheel={wheel} outside={outside}"
            );
            handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
            assert_eq!(state.annotations, before);
            assert_eq!(state.undo_stack.len(), 1);
        }
    }
}

/// 【标注命中测试】【旋转外接框】旋转矩形首次选择按Qt轴对齐外接框命中，框内空白不能改成反旋转后的矩形命中
#[test]
fn rotated_rectangle_first_selection_uses_the_visual_axis_aligned_bounds() {
    for wheel in [-1440, 1440] {
        let mut state = rectangle_editor(wheel);
        state.annotations[0].rotation_degrees = 45.0;
        let before = state.annotations.clone();
        let extent = 30.0 * 2.0_f64.sqrt();
        let position = state
            .canvas_transform()
            .to_screen((320.0 + extent - 1.0, 240.0 - extent + 1.0));
        let mut dirty = 0;
        pointer(&mut state, position.0, position.1, &mut dirty);
        handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
        assert_eq!(state.selected_annotation_index(), Some(0), "wheel={wheel}");
        handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
        assert_eq!(state.annotations, before);
        assert_eq!(state.undo_stack.len(), 1);
    }
}

/// 【标注命中测试】【序号窗口边界】尖端与圆心分离时首次圆框选择使用原图半径加八窗口像素，九像素点击不得消耗编号或历史
#[test]
fn initial_number_selection_uses_fixed_window_tolerance_and_retains_counter() {
    for wheel in [-1440, 1440] {
        for (outside, expected) in [(7.0, true), (9.0, false)] {
            let mut state = image_editor((1280, 960));
            let mut dirty = 0;
            select_tool(&mut state, Tool::Number, &mut dirty);
            pointer(&mut state, 320.0, 240.0, &mut dirty);
            handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
            pointer(&mut state, 220.0, 240.0, &mut dirty);
            handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
            select_tool(&mut state, Tool::Pick, &mut dirty);
            pointer(&mut state, 320.0, 240.0, &mut dirty);
            handle_axis_scroll(&mut state, ScrollEvent::wheel(wheel), &mut dirty);
            let crate::types::AnnotationShape::Number { mark } = state.annotations[0].shape else {
                unreachable!();
            };
            let radius = f64::from(mark.bubble_radius(state.annotations[0].stroke_width))
                * state.canvas_transform().scale;
            let before = state.annotations.clone();
            pointer(&mut state, 320.0 + radius + outside, 240.0, &mut dirty);
            handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
            assert_eq!(
                state.selected_annotation_index().is_some(),
                expected,
                "wheel={wheel} outside={outside}"
            );
            handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
            assert_eq!(state.annotations, before);
            assert_eq!(state.number_sequence.next(), Some(2));
            assert_eq!(state.undo_stack.len(), 1);
        }
    }
}

/// 【标注命中测试】【尺子工具职责】选择工具在长度尺内部整体移动，尺子工具在同一位置开始新测量，缩放不能改变职责
#[test]
fn ruler_interior_starts_a_new_measurement_only_in_the_ruler_tool() {
    for wheel in [-1440, 1440] {
        let mut state = image_editor((1280, 960));
        let mut dirty = 0;
        state.tool_settings.ruler_mode = crate::features::ruler::RulerMode::Length;
        select_tool(&mut state, Tool::Ruler, &mut dirty);
        pointer(&mut state, 220.0, 140.0, &mut dirty);
        handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
        pointer(&mut state, 420.0, 340.0, &mut dirty);
        handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
        select_tool(&mut state, Tool::Pick, &mut dirty);
        pointer(&mut state, 320.0, 240.0, &mut dirty);
        handle_axis_scroll(&mut state, ScrollEvent::wheel(wheel), &mut dirty);
        let before = state.annotations.clone();
        pointer(&mut state, 320.0, 240.0, &mut dirty);
        handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
        assert_eq!(
            state.ann_drag.as_ref().unwrap().handle,
            crate::types::SelectionHandle::Move
        );
        handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
        select_tool(&mut state, Tool::Ruler, &mut dirty);
        handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
        assert!(
            state.ann_drag.is_none() && state.pending.is_some(),
            "wheel={wheel}"
        );
        handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
        assert_eq!(state.annotations, before);
        assert_eq!(state.undo_stack.len(), 1);
    }
}

/// 【标注命中测试】【角度选框】真实角度尺的绘制、点击和缩放共用十四窗口像素留白，拖动后撤销恢复原始三点
#[test]
fn angle_frame_pixels_hit_and_resize_use_the_same_window_padding() {
    for wheel in [-1440, 1440] {
        let mut state = image_editor((1280, 960));
        let mut dirty = 0;
        state.tool_settings.ruler_mode = crate::features::ruler::RulerMode::Angle;
        select_tool(&mut state, Tool::Ruler, &mut dirty);
        for point in [(280.0, 220.0), (360.0, 220.0), (320.0, 260.0)] {
            pointer(&mut state, point.0, point.1, &mut dirty);
            handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
            handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
        }
        assert_eq!(state.annotations.len(), 1);
        select_tool(&mut state, Tool::Pick, &mut dirty);
        pointer(&mut state, 320.0, 240.0, &mut dirty);
        handle_axis_scroll(&mut state, ScrollEvent::wheel(wheel), &mut dirty);
        state.select_annotation_index(Some(0));
        let before = state.annotations.clone();
        let view = state.canvas_transform();
        let corner = view.to_screen((280.0, 260.0));
        let position = (corner.0 - 14.0, corner.1 + 14.0);
        let mut pixels = Pixmap::new(640, 480).unwrap();
        crate::renderer::draw_annotation(
            &mut crate::renderer::Painter::new(&mut pixels)
                .with_scale(view.scale as f32)
                .with_origin(view.origin((0.0, 0.0)))
                .with_window_units(state.hit_window_units()),
            &state.annotations[0],
            true,
            &mut std::collections::HashMap::new(),
            &std::collections::HashMap::new(),
            None,
        );
        let orange = (-5..=5)
            .flat_map(|y| (-5..=5).map(move |x| (x, y)))
            .filter(|(x, y)| {
                pixels
                    .pixel(
                        (position.0.round() as i32 + x) as u32,
                        (position.1.round() as i32 + y) as u32,
                    )
                    .is_some_and(|pixel| {
                        pixel.red() > 240
                            && pixel.green() > 130
                            && pixel.green() < 170
                            && pixel.blue() < 80
                    })
            })
            .count();
        assert!(
            orange > 20,
            "actual angle frame corner is missing: wheel={wheel} orange={orange}"
        );
        pointer(&mut state, position.0, position.1, &mut dirty);
        handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
        assert_eq!(
            state.ann_drag.as_ref().unwrap().handle,
            crate::types::SelectionHandle::BottomLeft
        );
        pointer(&mut state, position.0 - 6.0, position.1 + 5.0, &mut dirty);
        handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
        let crate::types::AnnotationShape::Ruler { measurement } = state.annotations[0].shape
        else {
            unreachable!();
        };
        let bounds = measurement.unrotated_bounds().unwrap();
        assert!((f64::from(bounds.left()) - (280.0 - 6.0 / view.scale)).abs() < 0.001);
        assert!((f64::from(bounds.bottom()) - (260.0 + 5.0 / view.scale)).abs() < 0.001);
        key(&mut state, "Ctrl+Z", &mut dirty);
        assert_eq!(state.annotations, before);
        assert_eq!(state.undo_stack.len(), 1);
    }
}
