//! 【尺子窗口测试】【Qt差分】完整生产布局逐项核对，预期来自只读Qt采集

use super::*;
use crate::features::{
    annotation_style::window::WindowUnits,
    ruler::{Measurement, RulerGeometry, tick_layout::Layout},
};
use serde::Deserialize;

#[derive(Deserialize)]
struct Reference {
    scenes: Vec<Scene>,
}

#[derive(Debug, Deserialize)]
struct Scene {
    rect: [f32; 4],
    scale: f32,
    capture: f32,
    width: f32,
    rotation: f32,
    context: usize,
    offset: (f32, f32),
    window: [f32; 4],
    selection: [f32; 4],
    allowed: [f32; 4],
    horizontal_band: [f32; 4],
    vertical_band: [f32; 4],
    scale_bounds: [f32; 4],
    below: bool,
    right: bool,
    clamp: bool,
    font_pixels: f32,
    top_reach: f32,
    step_x: u32,
    step_y: u32,
    connector_start: (f32, f32),
    rotation_handle: (f32, f32),
}

/// 【尺子窗口测试】【采集读取】无参数，返回环境指定的独立Qt采集；缺少采集时立即失败
fn reference() -> Reference {
    let path =
        std::env::var("MARK_SHOT_RULER_REFERENCE").expect("provide collected ruler reference.json");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

/// 【尺子窗口测试】【持久几何】按scene返回窗口坐标的测量和笔宽，截图补偿仅参与样式
fn measurement(scene: &Scene) -> Measurement {
    let [x, y, width, height] = scene.rect;
    Measurement {
        geometry: RulerGeometry::Length {
            start: (
                x * scene.scale + scene.offset.0,
                y * scene.scale + scene.offset.1,
            ),
            end: (
                (x + width) * scene.scale + scene.offset.0,
                (y + height) * scene.scale + scene.offset.1,
            ),
            rotation: scene.rotation,
        },
        pixel_scale: 1.0 / scene.scale,
        style_scale: 640.0 / (640.0 / f64::from(scene.capture)).round() * f64::from(scene.scale),
    }
}

/// 【尺子窗口测试】【矩形比较】按actual、expected和scene核对窗口矩形，误差超过0.02像素时失败，无返回值
fn rectangle(actual: Rect, expected: [f32; 4], scene: &Scene) {
    for (a, b) in [actual.x(), actual.y(), actual.width(), actual.height()]
        .into_iter()
        .zip(expected)
    {
        assert!(
            (a - b).abs() < 0.02,
            "actual={actual:?} expected={expected:?} scene={scene:?}"
        );
    }
}

/// 【尺子窗口测试】【完整布局矩阵】条带、换侧、允许范围、顶部留量和间隔使用同一Qt场景逐项比较
#[test]
#[ignore = "requires complete production Qt ruler reference capture"]
fn complete_qt_ruler_tick_layout_matches_all_window_contexts() {
    let reference = reference();
    assert_eq!(reference.scenes.len(), 10240);
    for scene in &reference.scenes {
        verify_layout(scene);
    }
}

/// 【尺子窗口测试】【单场景布局】按scene逐项比较Qt条带、字号和间隔，无返回值
fn verify_layout(scene: &Scene) {
    let [x, y, w, h] = scene.allowed;
    let layout = Layout::new(
        measurement(scene),
        scene.width * scene.scale,
        Rect::from_xywh(x, y, w, h),
        WindowUnits::default(),
    )
    .unwrap();
    rectangle(layout.horizontal_band, scene.horizontal_band, scene);
    rectangle(layout.vertical_band, scene.vertical_band, scene);
    rectangle(layout.scale_bounds().unwrap(), scene.scale_bounds, scene);
    assert_eq!(
        (layout.below, layout.right, layout.clamp),
        (scene.below, scene.right, scene.clamp),
        "{scene:?}"
    );
    assert_eq!(layout.font_pixels, scene.font_pixels, "{scene:?}");
    assert!(
        (layout.top_reach() - scene.top_reach).abs() < 0.02,
        "{scene:?}"
    );
    assert_eq!(
        (layout.step_x, layout.step_y),
        (scene.step_x, scene.step_y),
        "{scene:?}"
    );
}

/// 【尺子窗口测试】【原版连接矩阵】生产选中窗口连接点与完整Qt把手比较，保留平直和退化方向
#[test]
#[ignore = "requires complete production Qt ruler reference capture"]
fn complete_qt_ruler_rotation_connector_matches_all_window_contexts() {
    let reference = reference();
    let mut states = std::collections::HashMap::new();
    for scene in &reference.scenes {
        verify_connector(scene);
        let state = states
            .entry((scene.scale.to_bits(), scene.context))
            .or_insert_with(|| editor_state(scene));
        verify_editor_state(state, scene);
    }
}

/// 【尺子窗口测试】【单场景连接】按scene比较真实窗口连接点与Qt生产输出，无返回值
fn verify_connector(scene: &Scene) {
    let measurement = measurement(scene);
    let mut annotation = crate::types::Annotation {
        id: 7,
        style_scale: measurement.style_scale as f32,
        rotation_degrees: f64::from(scene.rotation),
        frozen_bounds: None,
        shape: crate::types::AnnotationShape::Ruler { measurement },
        color: Color::BLACK,
        shadow_color: Color::TRANSPARENT,
        stroke_width: scene.width * scene.scale,
        bbox: measurement.bounds().unwrap(),
    };
    annotation.update_bbox();
    let [x, y, w, h] = scene.allowed;
    let actual = crate::features::annotation_selection::rotation::window_connector_in_bounds(
        &annotation,
        WindowUnits::default(),
        Rect::from_xywh(x, y, w, h),
    )
    .unwrap();
    let error = |a: (f32, f32), b: (f32, f32)| (a.0 - b.0).hypot(a.1 - b.1);
    assert!(
        error(actual.0, scene.connector_start) < 0.02
            && error(actual.1, scene.rotation_handle) < 0.02,
        "actual={actual:?} context={} scene={scene:?}",
        scene.context
    );
}

/// 【尺子窗口测试】【固定Qt样本】常规测试读取81个独立Qt样本，覆盖临界贴边、平直和不同投影，不依赖本机Qt运行器
#[test]
fn stored_qt_ruler_samples_preserve_layout_and_rotation_regressions() {
    let reference: Reference = serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/qt-ruler-chrome/samples.json"
    ))
    .unwrap();
    assert_eq!(reference.scenes.len(), 81);
    for scene in &reference.scenes {
        verify_layout(scene);
        verify_connector(scene);
        verify_editor_bounds(scene);
    }
}

/// 【尺子窗口测试】【窗口状态对照】按scene提供相同选区与等比投影，核对编辑入口实际允许范围和连接点，无返回值
fn verify_editor_bounds(scene: &Scene) {
    verify_editor_state(&editor_state(scene), scene);
}

/// 【尺子窗口测试】【投影状态】按scene创建指定窗口与选区状态，返回编辑器，完整矩阵按相同投影复用
fn editor_state(scene: &Scene) -> EditorState {
    use crate::features::canvas_navigation::{coordinates, model::Navigation};
    let [x, y, w, h] = scene.window;
    let mut state = image_editor((640, 480));
    state.placements[0].size = (w as i32, h as i32);
    state.navigation.window = Rect::from_xywh(x, y, w, h);
    let model = Navigation::new((640, 480), (w as u32, h as u32), true).unwrap();
    let projected = model.projection().widget_rect;
    state.navigation.view = Some(model);
    state.navigation.reference = Some(coordinates::Rect {
        x: (projected.x - f64::from(scene.offset.0)) / f64::from(scene.scale),
        y: (projected.y - f64::from(scene.offset.1)) / f64::from(scene.scale),
        width: projected.width / f64::from(scene.scale),
        height: projected.height / f64::from(scene.scale),
    });
    let [sx, sy, sw, sh] = scene.selection;
    state.selection.zone = Rect::from_xywh(sx, sy, sw, sh);
    state
}

/// 【尺子窗口测试】【真实编辑入口】按state和scene比较允许范围与生产连接点，无返回值，不修改缓存状态
fn verify_editor_state(state: &EditorState, scene: &Scene) {
    let view = state.canvas_transform();
    rectangle(
        view.rect(state.ruler_allowed_bounds().unwrap()).unwrap(),
        scene.allowed,
        scene,
    );
    let window_measurement = measurement(scene);
    let mut source = window_measurement.mapped(|point| {
        (
            (point.0 - scene.offset.0) / scene.scale,
            (point.1 - scene.offset.1) / scene.scale,
        )
    });
    source.pixel_scale = 1.0;
    source.style_scale /= f64::from(scene.scale);
    let annotation = crate::types::Annotation {
        id: 7,
        style_scale: source.style_scale as f32,
        rotation_degrees: f64::from(scene.rotation),
        frozen_bounds: None,
        shape: crate::types::AnnotationShape::Ruler {
            measurement: source,
        },
        color: Color::BLACK,
        shadow_color: Color::TRANSPARENT,
        stroke_width: scene.width,
        bbox: source.bounds().unwrap(),
    };
    let (start, handle) = state.canvas_ruler_connector(&annotation).unwrap();
    for (actual, expected) in [
        (start, scene.connector_start),
        (handle, scene.rotation_handle),
    ] {
        let actual = view.to_screen((f64::from(actual.0), f64::from(actual.1)));
        assert!(
            (actual.0 - f64::from(expected.0)).hypot(actual.1 - f64::from(expected.1)) < 0.02,
            "editor connector actual={actual:?} expected={expected:?} {scene:?}"
        );
    }
}
