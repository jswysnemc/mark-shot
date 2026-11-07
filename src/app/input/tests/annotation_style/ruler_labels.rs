//! 【尺子说明测试】【完整Qt入口】窗口与导出说明使用完整生产drawLabels采集的尺寸、字号和位置

use super::*;
use crate::features::ruler::{Measurement, RulerGeometry, labels};
use serde::Deserialize;

#[derive(Deserialize)]
struct Reference {
    scenes: Vec<Scene>,
}

#[derive(Debug, Deserialize)]
struct Scene {
    serial: usize,
    widget: bool,
    scale: f64,
    capture: f32,
    pattern: usize,
    offset: (f64, f64),
    selection: [f32; 4],
    allowed: [f32; 4],
    inputs: Vec<Input>,
    labels: Vec<Label>,
    #[serde(default)]
    frame: String,
}

#[derive(Debug, Deserialize)]
struct Input {
    id: u64,
    angle: bool,
    rect: [f32; 4],
    points: Vec<(f32, f32)>,
    width: f32,
    rotation: f32,
}

#[derive(Debug, Deserialize)]
struct Label {
    rect: [f32; 4],
    text: String,
    unit: f32,
    font_pixels: f32,
}

/// 【尺子说明测试】【源图测量】按input和capture返回源图几何，截图补偿只参与样式单位
fn measurement(input: &Input, capture: f32) -> Measurement {
    let [x, y, width, height] = input.rect;
    Measurement {
        geometry: if input.angle {
            let mut points = [(0.0, 0.0); 3];
            for (a, b) in points.iter_mut().zip(&input.points) {
                *a = *b;
            }
            RulerGeometry::Angle {
                points,
                count: input.points.len().min(3) as u8,
            }
        } else {
            RulerGeometry::Length {
                start: (x, y),
                end: (x + width, y + height),
                rotation: input.rotation,
            }
        },
        pixel_scale: 1.0,
        style_scale: 640.0 / (640.0 / f64::from(capture)).round(),
    }
}

/// 【尺子说明测试】【窗口状态】按scene返回具有相同实际投影和选区的编辑器，草稿在窗口中走独立输入路径
fn editor_state(scene: &Scene) -> EditorState {
    use crate::features::canvas_navigation::{coordinates, model::Navigation};
    let mut state = image_editor((640, 480));
    let model = Navigation::new((640, 480), (640, 480), true).unwrap();
    let projected = model.projection().widget_rect;
    state.navigation.view = Some(model);
    state.navigation.window = Rect::from_xywh(0.0, 0.0, 640.0, 480.0);
    state.navigation.reference = Some(coordinates::Rect {
        x: (projected.x - scene.offset.0) / scene.scale,
        y: (projected.y - scene.offset.1) / scene.scale,
        width: projected.width / scene.scale,
        height: projected.height / scene.scale,
    });
    let [x, y, w, h] = scene.selection;
    state.selection.zone = Rect::from_xywh(x, y, w, h);
    for input in &scene.inputs {
        let measurement = measurement(input, scene.capture);
        let annotation = crate::types::Annotation {
            id: input.id,
            style_scale: measurement.style_scale as f32,
            rotation_degrees: f64::from(input.rotation),
            frozen_bounds: None,
            shape: crate::types::AnnotationShape::Ruler { measurement },
            color: Color::BLACK,
            shadow_color: Color::TRANSPARENT,
            stroke_width: input.width,
            bbox: measurement.bounds().unwrap(),
        };
        if scene.pattern >= 8 && input.id == 2 {
            state.pending = Some(annotation);
        } else {
            state.annotations.push(annotation);
        }
    }
    state
}

/// 【尺子说明测试】【生产说明比较】按scene比较真实说明缓存或导出布局，预期完全来自Qt，无返回值
fn verify(scene: &Scene) {
    let mut state = editor_state(scene);
    let view = if scene.widget {
        state.canvas_transform()
    } else {
        Default::default()
    };
    let actual = if scene.widget {
        let mut dirty = 0;
        crate::editor::ruler_labels::refresh(&mut state, &mut dirty);
        state.ruler.labels.clone()
    } else {
        let [x, y, w, h] = scene.allowed;
        labels::prepare_all(
            scene
                .inputs
                .iter()
                .map(|input| (measurement(input, scene.capture), input.width)),
            Rect::from_xywh(x, y, w, h).unwrap(),
            &mut state.text.font_system,
        )
    };
    assert_eq!(
        actual.len(),
        scene.labels.len(),
        "label count serial={} scene={scene:?}",
        scene.serial
    );
    for (actual, expected) in actual.iter().zip(&scene.labels) {
        assert_eq!(
            actual.lines.as_slice(),
            std::slice::from_ref(&expected.text),
            "serial={}",
            scene.serial
        );
        assert!(
            (f64::from(actual.font_size) * view.scale - f64::from(expected.font_pixels)).abs()
                < 0.02,
            "font pixels actual={} expected={} serial={} scene={scene:?}",
            f64::from(actual.font_size) * view.scale,
            expected.font_pixels,
            scene.serial
        );
        assert!(
            (f64::from(actual.unit) * view.scale - f64::from(expected.unit)).abs() < 0.02,
            "serial={}",
            scene.serial
        );
        let rect = view.rect(actual.rect).unwrap();
        for (a, b) in [rect.x(), rect.y(), rect.width(), rect.height()]
            .into_iter()
            .zip(expected.rect)
        {
            assert!(
                (a - b).abs() < 0.03,
                "actual={rect:?} expected={expected:?} serial={} scene={scene:?}",
                scene.serial
            );
        }
    }
    // 1. 【尺子说明测试】【光栅样本】显式指定新目录时输出与Qt同尺寸的透明说明层，用于独立视觉检查
    if !scene.frame.is_empty()
        && let Ok(root) = std::env::var("MARK_SHOT_RULER_LABEL_FRAMES")
    {
        let mut canvas = Pixmap::new(640, 480).unwrap();
        let origin = view.to_canvas((0.0, 0.0));
        let mut cache = cosmic_text::SwashCache::new();
        crate::renderer::ruler_overlay::draw_labels(
            &mut crate::renderer::Painter::new(&mut canvas)
                .with_scale(view.scale as f32)
                .with_origin((origin.0 as f32, origin.1 as f32))
                .with_window_units(crate::features::annotation_style::window::WindowUnits::new(
                    (1.0 / view.scale) as f32,
                ))
                .with_text(&mut state.text.font_system, &mut cache),
            &actual,
        );
        canvas
            .save_png(std::path::Path::new(&root).join(&scene.frame))
            .unwrap();
    }
}

/// 【尺子说明测试】【完整生产矩阵】窗口和导出共2880个场景逐项比较字号、尺寸、顺序及位置
#[test]
#[ignore = "requires complete production Qt ruler label reference capture"]
fn complete_qt_ruler_labels_match_window_and_export_entry_points() {
    let path = std::env::var("MARK_SHOT_RULER_LABEL_REFERENCE")
        .expect("provide complete Qt label reference.json");
    let reference: Reference = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(reference.scenes.len(), 2880);
    for scene in &reference.scenes {
        verify(scene);
    }
}

/// 【尺子说明测试】【固定生产样本】正常测试复核已采集的Qt字号、完整单行尺寸及临界避让位置
#[test]
fn captured_qt_ruler_labels_match_real_window_and_export_layout() {
    let reference: Reference = serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/qt-ruler-labels.json"
    ))
    .unwrap();
    assert_eq!(reference.scenes.len(), 99);
    for scene in &reference.scenes {
        verify(scene);
    }
}

/// 【尺子说明测试】【真实调整】实际拖动同一尺子时只有一条实时说明，局部生产帧与完整帧逐像素相同
#[test]
fn real_ruler_resize_keeps_one_live_label_and_exact_partial_frame() {
    let mut state = image_editor((640, 480));
    state.ui_hidden = true;
    let mut dirty = 0;
    draw(&mut state, Tool::Ruler, 6.0, &mut dirty);
    select_tool(&mut state, Tool::Pick, &mut dirty);
    pointer(&mut state, 300.0, 220.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    crate::editor::ruler_labels::refresh(&mut state, &mut dirty);
    super::navigation::render(&mut state, None);
    state.damage_rects.clear();
    state.layer_damage_rects.clear();
    state.selection.prev_zone = state.selection.zone;
    assert_eq!(state.ruler.labels.len(), 1);
    pointer(&mut state, 390.0, 300.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    pointer(&mut state, 414.0, 312.0, &mut dirty);
    assert!(state.pending.is_some());
    crate::editor::ruler_labels::refresh(&mut state, &mut dirty);
    assert_eq!(
        state.ruler.labels.len(),
        1,
        "same ruler was counted as both original and live pending object"
    );
    let area = state
        .monitor_dirty_rect(0)
        .expect("real resize must register damage");
    super::navigation::render(&mut state, Some(area));
    let partial = state.canvas[0].clone();
    super::navigation::render(&mut state, None);
    assert_eq!(
        partial.data(),
        state.canvas[0].data(),
        "real ruler label resize differs in partial frame: {area:?}"
    );
}

/// 【尺子说明测试】【调整顺序】两个真实尺子中调整第一项时，说明按原版持久数组顺序布局，不把实时项移到末尾
#[test]
fn real_ruler_resize_preserves_original_label_order() {
    let mut state = image_editor((640, 480));
    state.ui_hidden = true;
    let mut dirty = 0;
    state.tool_settings.ruler_mode = crate::features::ruler::RulerMode::Length;
    select_tool(&mut state, Tool::Ruler, &mut dirty);
    for (start, end) in [
        ((100.0, 80.0), (220.0, 180.0)),
        ((300.0, 260.0), (420.0, 360.0)),
    ] {
        pointer(&mut state, start.0, start.1, &mut dirty);
        handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
        pointer(&mut state, end.0, end.1, &mut dirty);
        handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    }
    let expected: Vec<_> = state
        .annotations
        .iter()
        .map(|annotation| annotation.id)
        .collect();
    assert_eq!(expected.len(), 2);
    select_tool(&mut state, Tool::Pick, &mut dirty);
    pointer(&mut state, 150.0, 130.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, false, &mut dirty);
    pointer(&mut state, 220.0, 180.0, &mut dirty);
    handle_pointer_button(&mut state, MouseButton::Left, true, &mut dirty);
    pointer(&mut state, 244.0, 192.0, &mut dirty);
    assert_eq!(state.pending.as_ref().unwrap().id, expected[0]);
    crate::editor::ruler_labels::refresh(&mut state, &mut dirty);
    let actual: Vec<_> = state
        .ruler
        .label_sources
        .iter()
        .map(|source| source.id)
        .collect();
    assert_eq!(
        actual, expected,
        "real live ruler label moved to the end of the source order"
    );
}
