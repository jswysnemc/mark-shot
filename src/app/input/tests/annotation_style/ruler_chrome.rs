//! 【尺子窗口测试】【生产布局】旋转位置按完整Qt输出比较，局部重绘经真实输入和帧入口验证

use super::*;
use crate::features::{
    annotation_selection::rotation,
    annotation_style::window::WindowUnits,
    ruler::{Measurement, RulerGeometry},
};
use serde::Deserialize;

#[derive(Deserialize)]
struct Reference {
    scenes: Vec<Scene>,
}

#[derive(Debug, Deserialize)]
struct Scene {
    index: usize,
    rect: [f32; 4],
    points: Vec<(f32, f32)>,
    scale: f32,
    width: f32,
    rotation: f32,
    rotation_handle: (f64, f64),
}

/// 【尺子窗口测试】【原图模型】按scene返回原图坐标中的尺子，窗口倍率不写入定点、样式和笔宽
fn annotation(scene: &Scene) -> crate::types::Annotation {
    let [x, y, width, height] = scene.rect;
    let measurement = Measurement {
        geometry: if scene.index == 11 {
            RulerGeometry::Length {
                start: (x, y),
                end: (x + width, y + height),
                rotation: scene.rotation,
            }
        } else {
            RulerGeometry::Angle {
                points: scene.points.clone().try_into().unwrap(),
                count: 3,
            }
        },
        pixel_scale: 1.0,
        style_scale: 1.0,
    };
    let mut annotation = crate::types::Annotation {
        id: 7,
        style_scale: 1.0,
        rotation_degrees: f64::from(scene.rotation),
        frozen_bounds: Rect::from_xywh(0.0, 0.0, 640.0, 480.0),
        shape: crate::types::AnnotationShape::Ruler { measurement },
        color: Color::BLACK,
        shadow_color: Color::TRANSPARENT,
        stroke_width: scene.width,
        bbox: Rect::from_xywh(x, y, width, height).unwrap(),
    };
    annotation.update_bbox();
    annotation
}

/// 【尺子窗口测试】【旋转生产位置】完整Qt旋转位置与窗口连接点逐项比较，误差按窗口像素评估，不用Rust公式构造预期
#[test]
#[ignore = "requires complete production Qt reference capture"]
fn ruler_rotation_handles_match_complete_qt_window_layout() {
    let path =
        std::env::var("MARK_SHOT_HIT_REFERENCE").expect("provide collected Qt reference.json");
    let reference: Reference = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    for scene in reference.scenes.iter().filter(|scene| scene.index >= 11) {
        let annotation = annotation(scene);
        let (_, actual) =
            rotation::window_connector(&annotation, WindowUnits::new(1.0 / scene.scale)).unwrap();
        let error = (f64::from(actual.0) - scene.rotation_handle.0)
            .hypot(f64::from(actual.1) - scene.rotation_handle.1)
            * f64::from(scene.scale);
        assert!(
            error < 0.01,
            "window_error={error} actual={actual:?} scene={scene:?}"
        );
    }
}

/// 【尺子窗口测试】【已提交帧】按state绘制完整生产帧后清理上次损坏，后续事件必须自己登记重绘区域，无返回值
fn settle(state: &mut EditorState) {
    let mut dirty = 0;
    crate::editor::ruler_labels::refresh(state, &mut dirty);
    let frame = crate::app::frame_render::Frame {
        active_text_id: None,
        scan_badge: None,
        current_color: Color::BLACK,
        outline_color: Color::TRANSPARENT,
        annotations_hidden: false,
    };
    crate::app::frame_render::rebuild_layer(state, 0, &frame);
    super::navigation::render(state, None);
    state.selection.prev_zone = state.selection.zone;
    state.damage_rects.clear();
    state.layer_damage_rects.clear();
}

/// 【尺子窗口测试】【真实旋转帧】创建后缩放并点击实际把手，旋转拖动使用登记损坏局部合成，与完整生产帧逐像素一致
#[test]
fn real_length_rotation_and_partial_frame_share_window_layout() {
    use crate::{
        app::input::handle_axis_scroll,
        types::{ScrollEvent, SelectionHandle},
    };
    for (size, device, wheel, flat, width) in [
        ((1280, 960), 1.0, -1440, false, 1.0),
        ((1280, 960), 1.5, 720, false, 12.0),
        ((320, 240), 2.0, -1440, false, 3.0),
        ((320, 240), 1.5, 720, false, 1.0),
        ((1280, 960), 1.5, -1440, true, 6.0),
        ((1280, 960), 1.0, 0, true, 6.0),
    ] {
        let mut state = image_editor(size);
        state.ui_hidden = true;
        state.placements[0].scale = device;
        state.canvas[0] = Pixmap::new((640.0 * device) as u32, (480.0 * device) as u32).unwrap();
        state.dimmed[0] = state.canvas[0].clone();
        state.annotations_layer[0] = state.canvas[0].clone();
        let mut dirty = 0;
        state.tool_settings.ruler_mode = crate::features::ruler::RulerMode::Length;
        select_tool(&mut state, Tool::Ruler, &mut dirty);
        state.settings_panel.visible = false;
        state.toolbar.opacity = 0.0;
        state.action_toolbar.opacity = 0.0;
        state.tool_settings.color = Color::BLACK;
        state.tool_settings.set_stroke(StrokeGroup::Ruler, width);
        pointer(&mut state, 260.0, 200.0, &mut dirty);
        handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
        pointer(
            &mut state,
            380.0,
            if flat { 200.0 } else { 270.0 },
            &mut dirty,
        );
        handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
        assert_eq!(state.annotations.len(), 1);
        select_tool(&mut state, Tool::Pick, &mut dirty);
        pointer(&mut state, 320.0, 240.0, &mut dirty);
        if wheel != 0 {
            handle_axis_scroll(&mut state, ScrollEvent::wheel(wheel), &mut dirty);
        }
        let view = state.canvas_transform();
        let body = view.to_screen((320.0, if flat { 200.0 } else { 235.0 }));
        pointer(&mut state, body.0, body.1, &mut dirty);
        handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
        handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
        assert_eq!(state.selected_annotation_index(), Some(0));
        crate::app::canvas_navigation::preview::clear(&mut state, &mut dirty);
        settle(&mut state);
        let (_, handle) = state.canvas_ruler_connector(&state.annotations[0]).unwrap();
        let handle = view.to_screen((f64::from(handle.0), f64::from(handle.1)));
        let pixel = state.canvas[0]
            .pixel(
                (handle.0 * f64::from(device)).round() as u32,
                (handle.1 * f64::from(device)).round() as u32,
            )
            .unwrap();
        assert!(
            pixel.red() > 220 && pixel.green() > 100 && pixel.green() < 190 && pixel.blue() < 100,
            "rendered handle differs from input handle: {pixel:?}, size={size:?} device={device} wheel={wheel} flat={flat}"
        );
        pointer(&mut state, handle.0, handle.1, &mut dirty);
        handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
        assert_eq!(
            state.ann_drag.as_ref().unwrap_or_else(|| panic!("rotation click failed: size={size:?} device={device} wheel={wheel} flat={flat} handle={handle:?} selected={:?} pending={:?}", state.selected_annotation_index(), state.pending)).handle,
            SelectionHandle::Rotate
        );
        pointer(&mut state, handle.0 + 8.0, handle.1 + 4.0, &mut dirty);
        crate::editor::ruler_labels::refresh(&mut state, &mut dirty);
        let area = state
            .monitor_dirty_rect(0)
            .expect("real rotation must register damage");
        super::navigation::render(&mut state, Some(area));
        let partial = state.canvas[0].clone();
        super::navigation::render(&mut state, None);
        let difference = partial
            .pixels()
            .iter()
            .zip(state.canvas[0].pixels())
            .position(|(a, b)| a != b);
        assert!(
            difference.is_none(),
            "size={size:?} device={device} wheel={wheel} flat={flat} area={area:?} first_pixel={difference:?}"
        );
        handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    }
}

/// 【尺子窗口测试】【角度草稿】按state和dirty通过三个真实单击创建角度尺，返回无值，不手工注入持久对象
fn angle(state: &mut EditorState, dirty: &mut u32) {
    state.tool_settings.ruler_mode = crate::features::ruler::RulerMode::Angle;
    select_tool(state, Tool::Ruler, dirty);
    for point in [(280.0, 220.0), (360.0, 220.0), (320.0, 260.0)] {
        pointer(state, point.0, point.1, dirty);
        handle_pointer_button(state, MouseButton::Left, true, dirty);
        handle_pointer_button(state, MouseButton::Left, false, dirty);
    }
    assert_eq!(state.annotations.len(), 1);
}

/// 【尺子窗口测试】【局部拖动帧】真实缩放后拖动角度选框，用生产损坏区域局部合成；结果应与同一状态的完整帧逐字节相同
#[test]
fn navigated_angle_resize_damage_matches_the_complete_production_frame() {
    use crate::{app::input::handle_axis_scroll, types::ScrollEvent};
    for wheel in [-1440, 1440] {
        let mut state = image_editor((1280, 960));
        let mut dirty = 0;
        angle(&mut state, &mut dirty);
        select_tool(&mut state, Tool::Pick, &mut dirty);
        pointer(&mut state, 320.0, 240.0, &mut dirty);
        handle_axis_scroll(&mut state, ScrollEvent::wheel(wheel), &mut dirty);
        let view = state.canvas_transform();
        let vertex = view.to_screen((280.0, 220.0));
        pointer(&mut state, vertex.0, vertex.1, &mut dirty);
        handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
        handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
        assert_eq!(state.selected_annotation_index(), Some(0));
        crate::app::canvas_navigation::preview::clear(&mut state, &mut dirty);
        settle(&mut state);
        let corner = view.to_screen((280.0, 260.0));
        let position = (corner.0 - 14.0, corner.1 + 14.0);
        pointer(&mut state, position.0, position.1, &mut dirty);
        handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
        pointer(&mut state, position.0 - 6.0, position.1 + 5.0, &mut dirty);
        assert!(state.pending.is_some());
        let area = state
            .monitor_dirty_rect(0)
            .expect("real drag must schedule an actual area");
        super::navigation::render(&mut state, Some(area));
        let partial = state.canvas[0].clone();
        super::navigation::render(&mut state, None);
        let mismatch = partial
            .pixels()
            .iter()
            .zip(state.canvas[0].pixels())
            .enumerate()
            .find(|(_, (actual, expected))| actual != expected);
        assert!(
            mismatch.is_none(),
            "wheel={wheel} area={area:?} first_pixel={mismatch:?}"
        );
    }
}
