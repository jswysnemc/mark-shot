//! 【标注选框测试】【留白交互】检查粗描边、旋转及缩放手势，不通过选框修改正文

use super::{annotation_selection::selected, pointer};
use crate::{
    app::input::handle_pointer_button,
    features::{annotation_selection::frame, annotation_style::window::WindowUnits},
    renderer::{Painter, draw_annotation},
    types::{AnnotationShape, MouseButton, SelectionHandle},
};
use tiny_skia::Pixmap;

/// 【标注选框测试】【留白绘制】不同笔宽和窗口倍率下，类矩形缩放点位于描边外，选中前后正文边缘保持原色
#[test]
fn rectangular_frames_preserve_stroke_pixels_at_each_window_scale() {
    let mut state = selected();
    for width in [1.5, 3.0, 40.0] {
        for unit in [0.5, 1.0, 2.0] {
            for shape in [
                AnnotationShape::Rectangle {
                    start: (180.0, 130.0),
                    end: (280.0, 210.0),
                    radius: 0.0,
                    filled: false,
                    focus: false,
                    fixed_ratio: false,
                },
                AnnotationShape::Circle {
                    start: (180.0, 130.0),
                    end: (280.0, 210.0),
                    filled: false,
                    focus: false,
                    fixed_ratio: false,
                },
                AnnotationShape::Marker {
                    start: (180.0, 130.0),
                    end: (280.0, 210.0),
                    kind: crate::types::MarkerShape::Square,
                    radius: 0.0,
                    filled: false,
                },
            ] {
                let annotation = &mut state.annotations[0];
                annotation.shape = shape;
                annotation.stroke_width = width;
                annotation.update_bbox();
                let before = annotation.clone();
                let window = WindowUnits::new(unit);
                let bounds = frame::bounds(annotation, window).unwrap();
                let stroke_edge = 130.0 - window.stroke(width, 1.5, None) * 0.5;
                assert!((window.window(stroke_edge - bounds.top()) - 6.0).abs() < 0.0001);
                let mut body = Pixmap::new(640, 480).unwrap();
                draw_annotation(
                    &mut Painter::new(&mut body).with_window_units(window),
                    annotation,
                    false,
                    &mut Default::default(),
                    &Default::default(),
                    None,
                );
                let mut selected = Pixmap::new(640, 480).unwrap();
                draw_annotation(
                    &mut Painter::new(&mut selected).with_window_units(window),
                    annotation,
                    true,
                    &mut Default::default(),
                    &Default::default(),
                    None,
                );
                assert_eq!(selected.pixel(230, 130), body.pixel(230, 130));
                assert_ne!(body.pixel(230, 130).unwrap().alpha(), 0);
                assert_eq!(*annotation, before);
            }
        }
    }
}

/// 【标注选框测试】【缩放偏移】从扩大后右边控制点拖动，静止不改变正文，实际位移只调整对应正文边缘
#[test]
fn padded_resize_keeps_the_body_anchor_and_drag_distance() {
    for degrees in [0.0, 45.0, 90.0] {
        let mut state = selected();
        state.annotations[0].stroke_width = 40.0;
        state.annotations[0].rotation_degrees = degrees;
        state.annotations[0].update_bbox();
        let before = state.annotations[0].clone();
        let start = before.to_visual((306.0, 170.0));
        let end = before.to_visual((326.0, 170.0));
        assert_eq!(
            state.canvas_handle(&before, (start.0 as f64, start.1 as f64)),
            SelectionHandle::Right
        );
        pointer(&mut state, start.0 as f64, start.1 as f64, &mut 0);
        handle_pointer_button(&mut state, MouseButton::Left, true, &mut 0);
        pointer(&mut state, start.0 as f64, start.1 as f64, &mut 0);
        handle_pointer_button(&mut state, MouseButton::Left, false, &mut 0);
        assert_eq!(state.annotations[0], before);
        assert!(state.undo_stack.is_empty());
        handle_pointer_button(&mut state, MouseButton::Left, true, &mut 0);
        pointer(&mut state, end.0 as f64, end.1 as f64, &mut 0);
        handle_pointer_button(&mut state, MouseButton::Left, false, &mut 0);
        let AnnotationShape::Rectangle { start, end, .. } = state.annotations[0].shape else {
            panic!("rectangle required")
        };
        assert_eq!(start, (180.0, 130.0));
        assert!((end.0 - 300.0).abs() < 0.001 && (end.1 - 210.0).abs() < 0.001);
        assert_eq!(state.undo_stack.len(), 1);
        state.undo(&mut 0);
        assert_eq!(state.annotations[0], before);
    }
}
