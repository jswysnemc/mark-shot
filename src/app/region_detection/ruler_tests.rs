use super::*;
use crate::{
    features::ruler::{RulerGeometry, RulerMode},
    types::AnnotationShape,
};

/// 【尺子测试】【外框吸附】从把手边缘调整尺子，首个像素只改变一像素，接近候选角点时只吸附真实测量点
#[test]
fn padded_ruler_handle_resizes_and_snaps_the_measured_corner() {
    use crate::types::SelectionHandle;
    let mut state = editor(true, false, 1.0);
    crate::app::input::select_tool(&mut state, Tool::Ruler, &mut 0);
    state.tool_settings.ruler_mode = RulerMode::Length;
    pointer(&mut state, 120.0, 100.0, &mut 0);
    button(&mut state, MouseButton::Left, true, &mut 0);
    pointer(&mut state, 200.0, 140.0, &mut 0);
    button(&mut state, MouseButton::Left, false, &mut 0);
    state.select_annotation_index(Some(0));
    let original = state.annotations[0].clone();
    let frame = state.canvas_ruler_frame(&original).unwrap().rect;
    let grab = (
        f64::from(frame.right()) + 2.0,
        f64::from(frame.bottom()) + 1.0,
    );
    assert!(grab.0 > 210.0 && grab.1 > 150.0);
    pointer(&mut state, grab.0, grab.1, &mut 0);
    button(&mut state, MouseButton::Left, true, &mut 0);
    assert_eq!(
        state.ann_drag.as_ref().unwrap().handle,
        SelectionHandle::BottomRight
    );
    for (delta, end) in [((1.0, 1.0), (201.0, 141.0)), ((38.0, 38.0), (240.0, 180.0))] {
        pointer(&mut state, grab.0 + delta.0, grab.1 + delta.1, &mut 0);
        let AnnotationShape::Ruler { measurement } = state.pending.as_ref().unwrap().shape else {
            unreachable!()
        };
        assert_eq!(
            measurement.geometry,
            RulerGeometry::Length {
                start: (120.0, 100.0),
                end,
                rotation: 0.0
            }
        );
    }
    button(&mut state, MouseButton::Left, false, &mut 0);
    state.undo(&mut 0);
    assert_eq!(state.annotations, [original]);
}

/// 【尺子测试】【长边对齐】小控件位于箭头两端及中点之外时，重叠的尺边仍可吸附；边的延长线不能吸附
#[test]
fn long_arrow_snaps_only_where_its_axis_overlaps_a_region() {
    use crate::features::ruler::{Measurement, RulerStyle};
    let mut state = editor(true, false, 1.0);
    crate::app::input::select_tool(&mut state, Tool::Ruler, &mut 0);
    let mut ruler = Measurement::new(RulerMode::Length, (5.0, 73.0), 1.0)
        .unwrap()
        .with_style(RulerStyle::Simple);
    for (end, expected) in [(620.0, 80.0), (75.0, 77.0)] {
        ruler.geometry = RulerGeometry::Length {
            start: (5.0, 73.0),
            end: (end, 73.0),
            rotation: 0.0,
        };
        assert_eq!(
            ruler_move::pointer(&mut state, ruler, (50.0, 73.0), (50.0, 77.0)),
            (50.0, expected)
        );
    }
}

/// 【尺子测试】【整尺移动】拖动尺身时按角点或箭头端点吸附，远离后自由移动；两种工具入口均保留测量形状与单步撤销
#[test]
fn dragging_ruler_bodies_snaps_geometry_not_the_grab_point() {
    use crate::features::ruler::RulerStyle;
    for (mode, style) in [
        (RulerMode::Length, RulerStyle::Standard),
        (RulerMode::Length, RulerStyle::Simple),
        (RulerMode::Angle, RulerStyle::Standard),
    ] {
        for tool in [Tool::Ruler, Tool::Pick] {
            let mut state = editor(true, false, 1.5);
            state.tool_settings.ruler_mode = mode;
            state.tool_settings.ruler_style = style;
            crate::app::input::select_tool(&mut state, Tool::Ruler, &mut 0);
            pointer(&mut state, 120.0, 110.0, &mut 0);
            button(&mut state, MouseButton::Left, true, &mut 0);
            pointer(
                &mut state,
                220.0,
                if mode == RulerMode::Angle {
                    110.0
                } else {
                    152.0
                },
                &mut 0,
            );
            button(&mut state, MouseButton::Left, false, &mut 0);
            if mode == RulerMode::Angle {
                pointer(&mut state, 120.0, 152.0, &mut 0);
                button(&mut state, MouseButton::Left, true, &mut 0);
                button(&mut state, MouseButton::Left, false, &mut 0);
            }
            let original = state.annotations[0].clone();
            let AnnotationShape::Ruler { measurement } = original.shape else {
                panic!("expected ruler")
            };
            if tool == Tool::Pick {
                crate::app::input::select_tool(&mut state, tool, &mut 0);
            }
            let (grab, shift) = if style == RulerStyle::Simple {
                (131.0, 49.0)
            } else {
                (110.0, 28.0)
            };
            pointer(&mut state, 144.0, grab, &mut 0);
            button(&mut state, MouseButton::Left, true, &mut 0);
            assert_eq!(
                state.ann_drag.as_ref().unwrap().handle,
                crate::types::SelectionHandle::Move
            );
            pointer(&mut state, 161.0, grab + shift - 3.0, &mut 0);
            let expected = measurement.mapped(|point| (point.0 + 20.0, point.1 + shift as f32));
            assert_eq!(
                state.pending.as_ref().unwrap().shape,
                AnnotationShape::Ruler {
                    measurement: expected
                },
                "{mode:?} {style:?} {tool:?}"
            );
            pointer(&mut state, 250.0, grab + 110.0, &mut 0);
            let free = measurement.mapped(|point| (point.0 + 106.0, point.1 + 110.0));
            assert_eq!(
                state.pending.as_ref().unwrap().shape,
                AnnotationShape::Ruler { measurement: free }
            );
            button(&mut state, MouseButton::Left, false, &mut 0);
            state.undo(&mut 0);
            assert_eq!(state.annotations, [original]);
        }
    }
}

/// 【尺子测试】【区域吸附】在不同截图比例下验证两侧接近边角、区域内部自由测量及窗口回退，不改变截图选区
#[test]
fn ruler_snaps_to_detected_edges_without_changing_the_selection() {
    for scale in [1.0, 1.5] {
        for (image, windows, start, end) in [
            (true, true, (100.0, 80.0), (240.0, 180.0)),
            (false, true, (98.0, 83.0), (243.0, 178.0)),
            (false, false, (98.0, 83.0), (243.0, 178.0)),
        ] {
            let mut state = editor(image, windows, scale);
            state.tool_settings.ruler_mode = RulerMode::Length;
            crate::app::input::select_tool(&mut state, Tool::Ruler, &mut 0);
            let selection = state.selection.zone;
            pointer(&mut state, 98.0, 83.0, &mut 0);
            assert_eq!(state.ruler.hover.as_ref().unwrap().point, start);
            button(&mut state, MouseButton::Left, true, &mut 0);
            pointer(&mut state, 243.0, 178.0, &mut 0);
            button(&mut state, MouseButton::Left, false, &mut 0);
            let AnnotationShape::Ruler { measurement } = state.annotations[0].shape else {
                panic!("expected ruler")
            };
            assert_eq!(
                measurement.geometry,
                RulerGeometry::Length {
                    start,
                    end,
                    rotation: 0.0
                }
            );
            assert_eq!(state.selection.zone, selection);
            assert_eq!(state.undo_stack.len(), 1);
            pointer(&mut state, 156.0, 126.0, &mut 0);
            assert_eq!(state.ruler.hover.as_ref().unwrap().point, (156.0, 126.0));
            if windows {
                pointer(&mut state, 43.0, 140.0, &mut 0);
                assert_eq!(state.ruler.hover.as_ref().unwrap().point, (40.0, 140.0));
            }
        }
    }
}

/// 【尺子测试】【视图倍率】截图和文件均按屏幕容差吸附，文件候选使用原件比例和视口偏移，不吸附到桌面坐标
#[test]
fn zoomed_ruler_uses_window_tolerance_and_file_coordinates() {
    for file in [false, true] {
        let mut state = editor(true, false, 1.0);
        if file {
            state.file_canvas = Some(
                crate::editor::file_canvas::FileCanvas::new(
                    Pixmap::new(640, 480).unwrap(),
                    Rect::from_xywh(80.0, 40.0, 480.0, 360.0).unwrap(),
                )
                .unwrap(),
            );
        }
        let space = geometry::Space::for_editor(&state).unwrap();
        state.region_detection.space = Some(space);
        state.selection.zone = Some(space.bounds);
        state.tool_settings.ruler_mode = RulerMode::Angle;
        crate::app::input::select_tool(&mut state, Tool::Ruler, &mut 0);
        let rect = space.global(PixelRect::new(100, 80, 140, 100)).unwrap();
        let anchor = (f64::from(rect.left()), f64::from(rect.top()));
        assert!(state.ensure_navigation());
        state
            .navigation
            .view
            .as_mut()
            .unwrap()
            .zoom_at(2.0, anchor, true);
        let view = state.canvas_transform();
        let center = view.to_screen((anchor.0, anchor.1 + 20.0));
        pointer(&mut state, center.0 + 4.0, center.1, &mut 0);
        assert_eq!(
            state.ruler.hover.as_ref().unwrap().point,
            (rect.left(), rect.top() + 20.0)
        );
        button(&mut state, MouseButton::Left, true, &mut 0);
        button(&mut state, MouseButton::Left, false, &mut 0);
        pointer(&mut state, center.0 + 8.0, center.1 + 30.0, &mut 0);
        let AnnotationShape::Ruler { measurement } = state.pending.as_ref().unwrap().shape else {
            panic!("expected angle")
        };
        let RulerGeometry::Angle { points, .. } = measurement.geometry else {
            panic!("expected angle")
        };
        assert_eq!(points[0].0, rect.left());
        assert_eq!(points[1].0, rect.left() + 4.0);
    }
}
