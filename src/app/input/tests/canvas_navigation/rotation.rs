//! 【画布导航测试】【旋转投影】验证旋转连接点始终保留窗口间隔，真实绘制与指针命中一致

use super::{draw, image_editor};
use crate::{
    app::input::{handle_pointer_button, select_tool},
    tools::Tool,
    types::{Annotation, AnnotationShape, MouseButton, SelectionHandle},
};
use tiny_skia::{Color, Rect};

/// 【画布导航测试】【旋转标注】按degrees返回中心附近的矩形，保留独立原图边界与稳定几何
fn rectangle(degrees: f64) -> Annotation {
    let mut annotation = Annotation {
        style_scale: 1.0,
        id: 42,
        shape: AnnotationShape::Rectangle {
            start: (312.0, 235.0),
            end: (328.0, 245.0),
            filled: false,
        },
        rotation_degrees: degrees,
        frozen_bounds: Rect::from_xywh(0.0, 0.0, 640.0, 480.0),
        color: Color::BLACK,
        shadow_color: Color::TRANSPARENT,
        stroke_width: 3.0,
        bbox: Rect::from_xywh(312.0, 235.0, 16.0, 10.0).unwrap(),
    };
    annotation.update_bbox();
    annotation
}

/// 【画布导航测试】【倍率准备】按zoom与scale返回真实图片编辑器，直接修改独立视图，不改变标注坐标
fn navigated(zoom: f64, scale: f32) -> crate::editor::EditorState {
    let mut state = image_editor(scale);
    assert!(state.ensure_navigation());
    state
        .navigation
        .view
        .as_mut()
        .unwrap()
        .zoom_at(zoom, (320.0, 240.0), true);
    state
}

/// 【画布导航测试】【窗口旋转点】按top、handle和view返回固定26窗口像素的预期位置，不使用实现中的反向比例
fn screen_handle(
    top: (f32, f32),
    handle: (f32, f32),
    view: crate::editor::canvas_navigation::transform::ViewTransform,
) -> (f64, f64) {
    let direction = (f64::from(handle.0 - top.0), f64::from(handle.1 - top.1));
    let length = direction.0.hypot(direction.1);
    let top = view.to_screen((f64::from(top.0), f64::from(top.1)));
    (
        top.0 + direction.0 / length * 26.0,
        top.1 + direction.1 / length * 26.0,
    )
}

/// 【画布导航测试】【单项窗口间隔】各倍率和旋转角度下，圆点在固定窗口位置接受真实按下，旧倍率位置不再命中旋转
#[test]
fn ordinary_rotation_keeps_the_window_gap_and_pointer_target() {
    for zoom in [0.25, 0.5, 1.0, 2.0, 8.0, 16.0] {
        for degrees in [0.0, 45.0, 120.0, 180.0] {
            let mut state = navigated(zoom, 1.5);
            let mut dirty = 0;
            select_tool(&mut state, Tool::Pick, &mut dirty);
            let annotation = rectangle(degrees);
            let (top, handle) = annotation.generic_rotation_handle().unwrap();
            state.annotations.push(annotation.clone());
            state.select_annotation_index(Some(0));
            let view = state.canvas_transform();
            let screen = screen_handle(top, handle, view);
            let point = view.to_canvas(screen);
            assert_eq!(
                state.canvas_handle(&annotation, point),
                SelectionHandle::Rotate,
                "zoom={zoom}, degrees={degrees}"
            );
            if zoom >= 2.0 {
                assert_ne!(
                    state.canvas_handle(&annotation, (f64::from(handle.0), f64::from(handle.1))),
                    SelectionHandle::Rotate,
                    "old projection, zoom={zoom}, degrees={degrees}"
                );
            }
            super::super::pointer(&mut state, screen.0, screen.1, &mut dirty);
            handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
            assert_eq!(
                state.ann_drag.as_ref().unwrap().handle,
                SelectionHandle::Rotate
            );
            handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
            assert!(state.undo_stack.is_empty());
        }
    }
}

/// 【画布导航测试】【旋转实际画面】真实帧在预期圆心绘制橙色点，缩放后的旧圆心保持原图，反复局部重绘与整帧一致
#[test]
fn rotation_pixels_use_the_fixed_window_position_and_damage_grid() {
    for scale in [1.0, 1.5, 2.0] {
        let mut state = navigated(2.0, scale);
        let annotation = rectangle(45.0);
        let (top, handle) = annotation.generic_rotation_handle().unwrap();
        state.annotations.push(annotation);
        state.select_annotation_index(Some(0));
        let view = state.canvas_transform();
        let screen = screen_handle(top, handle, view);
        draw(&mut state, None);
        let canvas = &state.canvas[0];
        let pixel = canvas
            .pixel(
                (screen.0 * f64::from(scale)).round() as u32,
                (screen.1 * f64::from(scale)).round() as u32,
            )
            .unwrap();
        assert_eq!(
            (pixel.red(), pixel.green(), pixel.blue()),
            (251, 146, 60),
            "device scale={scale}"
        );
        let old = view.to_screen((f64::from(handle.0), f64::from(handle.1)));
        let pixel = canvas
            .pixel(
                (old.0 * f64::from(scale)).round() as u32,
                (old.1 * f64::from(scale)).round() as u32,
            )
            .unwrap();
        assert_ne!((pixel.red(), pixel.green(), pixel.blue()), (251, 146, 60));
        let expected = canvas.data().to_vec();
        let dirty =
            Rect::from_xywh(screen.0 as f32 - 10.0, screen.1 as f32 - 10.0, 20.0, 20.0).unwrap();
        for _ in 0..3 {
            draw(&mut state, Some(dirty));
        }
        assert_eq!(state.canvas[0].data(), expected);
    }
}

/// 【画布导航测试】【群组窗口间隔】放大和缩小后，群组旋转点仍在上边26窗口像素处，实际按下开始共同旋转
#[test]
fn collection_rotation_uses_the_same_screen_gap() {
    for zoom in [0.25, 0.5, 1.0, 2.0, 8.0, 16.0] {
        let mut state = navigated(zoom, 1.5);
        let mut dirty = 0;
        select_tool(&mut state, Tool::Pick, &mut dirty);
        let first = rectangle(0.0);
        let mut second = first.clone();
        second.id = 43;
        second.translate_mut(2.0, 4.0);
        state.annotations.extend([first, second]);
        state.select_annotation_ids([42, 43]);
        let bounds = state.collection_bounds().unwrap();
        let top = state.canvas_transform().to_screen((
            f64::from(bounds.left() + bounds.right()) * 0.5,
            f64::from(bounds.top()),
        ));
        let screen = (top.0, top.1 - 26.0);
        let point = state.canvas_transform().to_canvas(screen);
        assert_eq!(
            state.canvas_collection_handle(bounds, point),
            SelectionHandle::Rotate,
            "zoom={zoom}"
        );
        super::super::pointer(&mut state, screen.0, screen.1, &mut dirty);
        handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
        assert_eq!(
            state.collection.drag.as_ref().unwrap().handle,
            SelectionHandle::Rotate
        );
        handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
        assert!(state.undo_stack.is_empty());
    }
}

/// 【画布导航测试】【专用模型】无参数，返回线条、滤镜、序号、放大镜及两类尺子，覆盖各自连接起点与方向
fn specialties() -> Vec<Annotation> {
    use crate::features::{
        filter::{FilterMode, RegionFilter},
        magnifier::{Magnifier, MagnifierShape},
        number::{NumberMark, NumberStyle},
        ruler::{Measurement, RulerGeometry, RulerMode},
    };
    let filter =
        RegionFilter::new((312.0, 235.0), (328.0, 245.0), 14.0, FilterMode::Pixelate).unwrap();
    let mut mark = NumberMark::new((320.0, 240.0), 1, NumberStyle::Arabic).unwrap();
    mark.rotation = 0.6;
    let mut magnifier = Magnifier::new(
        MagnifierShape::Rectangle,
        (316.0, 240.0),
        2.0,
        Rect::from_xywh(0.0, 0.0, 640.0, 480.0).unwrap(),
        1.0,
    )
    .unwrap();
    magnifier.center = (324.0, 240.0);
    magnifier.size = (16.0, 12.0);
    magnifier.rotation = 0.6;
    let mut length = Measurement::new(RulerMode::Length, (312.0, 235.0), 1.0).unwrap();
    length.geometry = RulerGeometry::Length {
        start: (312.0, 235.0),
        end: (328.0, 245.0),
        rotation: 45.0,
    };
    let mut angle = Measurement::new(RulerMode::Angle, (312.0, 240.0), 1.0).unwrap();
    angle.geometry = RulerGeometry::Angle {
        points: [(312.0, 240.0), (328.0, 235.0), (328.0, 245.0)],
        count: 3,
    };
    let line = crate::features::line::LineSkeleton::new((312.0, 235.0), (328.0, 245.0)).unwrap();
    [
        AnnotationShape::Line {
            line,
            style: Default::default(),
        },
        AnnotationShape::Redact { filter },
        AnnotationShape::Number { mark },
        AnnotationShape::Magnifier { magnifier },
        AnnotationShape::Ruler {
            measurement: length,
        },
        AnnotationShape::Ruler { measurement: angle },
    ]
    .into_iter()
    .map(|shape| {
        let mut annotation = rectangle(0.0);
        annotation.shape = shape;
        annotation.update_bbox();
        annotation
    })
    .collect()
}

/// 【画布导航测试】【专用旋转一致】缩放后选择工具的旋转命中与各专用绘制器实际圆心一致，不把既有间隔随图片放大
#[test]
fn specialty_rotation_matches_the_actual_drawn_circle() {
    use crate::features::annotation_selection::rotation;
    for zoom in [0.5, 2.0, 4.0] {
        let state = navigated(zoom, 1.5);
        let view = state.canvas_transform();
        for annotation in specialties() {
            let (_, end) =
                rotation::window_connector(&annotation, state.hit_window_units()).unwrap();
            let screen = view.to_screen((f64::from(end.0), f64::from(end.1)));
            assert_eq!(
                state.canvas_handle(&annotation, view.to_canvas(screen)),
                SelectionHandle::Rotate,
                "zoom={zoom}, shape={:?}",
                annotation.shape
            );
            let mut canvas = tiny_skia::Pixmap::new(960, 720).unwrap();
            let mut painter = crate::renderer::Painter::new(&mut canvas)
                .with_scale(1.5 * zoom as f32)
                .with_origin(view.origin((0.0, 0.0)))
                .with_chrome_scale(1.0 / zoom as f32);
            crate::renderer::draw_annotation(
                &mut painter,
                &annotation,
                true,
                &mut Default::default(),
                &Default::default(),
                None,
            );
            let pixel = canvas
                .pixel(
                    (screen.0 * 1.5).round() as u32,
                    (screen.1 * 1.5).round() as u32,
                )
                .unwrap();
            assert_eq!(
                (pixel.red(), pixel.green(), pixel.blue()),
                (251, 146, 60),
                "zoom={zoom}, shape={:?}",
                annotation.shape
            );
        }
    }
}
