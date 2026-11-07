//! 【文字存储测试】【完整生产对照】逐项核验原始矩形、原图选框、点字号与预览字形尺寸

use super::*;
use serde_json::Value;
use tiny_skia::{Color, Rect};

/// 【文字存储测试】【生产样本】返回由完整Qt生产函数采集的记录，无参数，不重新生成预期
fn reference() -> Value {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/qt-text-storage/reference.json"
    )))
    .unwrap()
}

/// 【文字存储测试】【几何读取】按value解码左上宽高，返回逻辑矩形，拒绝无效样本
fn rect(value: &Value) -> Rect {
    Rect::from_xywh(
        value[0].as_f64().unwrap() as f32,
        value[1].as_f64().unwrap() as f32,
        value[2].as_f64().unwrap() as f32,
        value[3].as_f64().unwrap() as f32,
    )
    .unwrap()
}

/// 【文字存储测试】【场景准备】按raw和content返回Noto Sans二十点文字及独立字体状态，不连接桌面
fn prepared(raw: Rect, content: &str) -> (Annotation, TextState) {
    let annotation = Annotation {
        style_scale: 1.0,
        id: 9,
        rotation_degrees: 0.0,
        frozen_bounds: Some(Rect::from_xywh(0.0, 0.0, 640.0, 480.0).unwrap()),
        shape: AnnotationShape::Text {
            start: (raw.left(), raw.top()),
            content: content.into(),
            font_size: 20.0,
            bold: true,
            italic: false,
            outline: None,
            frame: crate::features::text::TextFrame {
                width: Some(raw.width()),
                height: Some(raw.height()),
                anchor: Some((raw.left(), raw.top())),
                painted: None,
            },
            appearance: crate::features::text::TextAppearance {
                family: "Noto Sans".into(),
                ..Default::default()
            },
        },
        color: Color::BLACK,
        shadow_color: Color::TRANSPARENT,
        stroke_width: 0.0,
        bbox: raw,
    };
    let text = TextState {
        font_system: crate::fonts::bundled(),
        system_fonts: None,
        swash_cache: cosmic_text::SwashCache::new(),
        editors: Default::default(),
        editing: None,
    };
    (annotation, text)
}

/// 【文字存储测试】【样本断言】按annotation、expected和context比较真实存储尺寸、原图框与点字号，无返回值
fn assert_state(annotation: &Annotation, expected: &Value, context: &str) {
    let AnnotationShape::Text {
        start,
        frame,
        font_size,
        ..
    } = &annotation.shape
    else {
        unreachable!()
    };
    let actual = [
        start.0,
        start.1,
        frame.width.unwrap(),
        frame.height.unwrap(),
    ];
    for (index, value) in actual.into_iter().enumerate() {
        let expected = expected["stored"][index].as_f64().unwrap();
        assert!(
            (f64::from(value) - expected).abs() < 0.002,
            "{context} stored[{index}]: {value} != {expected}"
        );
    }
    for (index, value) in [annotation.bbox.width(), annotation.bbox.height()]
        .into_iter()
        .enumerate()
    {
        let expected = expected["source"][index].as_f64().unwrap();
        assert!(
            (f64::from(value) - expected).abs() < 0.002,
            "{context} source[{index}]: {value} != {expected}"
        );
    }
    assert!(
        (f64::from(*font_size) - expected["points"].as_f64().unwrap()).abs() < 0.002,
        "{context}: point size mismatch"
    );
    if let Some(anchor) = expected["anchor"].as_array() {
        let actual = frame.anchor.unwrap_or(*start);
        assert!(
            (f64::from(actual.0) - anchor[0].as_f64().unwrap()).abs() < 0.002,
            "{context} anchor x"
        );
        assert!(
            (f64::from(actual.1) - anchor[1].as_f64().unwrap()).abs() < 0.002,
            "{context} anchor y"
        );
    }
    if !expected["source_frame"].is_null() {
        let expected = rect(&expected["source_frame"]);
        assert!(
            (annotation.bbox.left() - expected.left()).abs() < 0.002,
            "{context}: source x"
        );
        assert!(
            (annotation.bbox.top() - expected.top()).abs() < 0.002,
            "{context}: source y"
        );
    }
}

/// 【文字存储测试】【完整八向变换】144个生产样本包括已有隐藏宽高、原图边缘和等比例目标，每项逐字段比较，无参数、无返回值
#[test]
fn all_production_storage_transforms_match() {
    let reference = reference();
    let handles = [
        SelectionHandle::TopLeft,
        SelectionHandle::Top,
        SelectionHandle::TopRight,
        SelectionHandle::Right,
        SelectionHandle::BottomRight,
        SelectionHandle::Bottom,
        SelectionHandle::BottomLeft,
        SelectionHandle::Left,
    ];
    let samples = reference["single"].as_array().unwrap();
    assert_eq!(samples.len(), 144);
    for (index, sample) in samples.iter().enumerate() {
        let (mut annotation, mut text) = prepared(rect(&sample["before"]), "Rotation");
        let editor = crate::tools::text::ensure_text_editor(
            &annotation,
            &mut text.editors,
            &mut text.font_system,
        );
        crate::tools::text::update_text_bbox_inline(&mut annotation, editor, &mut text.font_system);
        assert_eq!(
            chrome::bounds(&annotation).unwrap(),
            rect(&sample["frame"]),
            "sample={index} input frame"
        );
        let before = annotation.clone();
        text_resize(
            &mut annotation,
            &before,
            handles[sample["handle"].as_u64().unwrap() as usize],
            rect(&sample["target"]),
            drag::Options {
                bounds: before.frozen_bounds.unwrap(),
                pixel_scale: 1.0,
                keep_aspect: sample["aspect"].as_bool().unwrap(),
                snap_rotation: false,
            },
            &mut text,
        );
        assert_state(&annotation, &sample["after"], &format!("sample={index}"));
        assert_eq!(
            text.editors[&annotation.id]
                .with_buffer(|buffer| buffer.wrap() != cosmic_text::Wrap::None),
            sample["after"]["wraps"].as_bool().unwrap(),
            "sample={index} wrapping"
        );
    }
}

/// 【文字存储测试】【连续变换收框】六个生产序列的每步独立读取可见框，最后按换行状态收紧存储矩形，无参数、无返回值
#[test]
fn complete_sequences_and_final_fit_match() {
    for sample in reference()["sequences"].as_array().unwrap() {
        let (mut annotation, mut text) = prepared(
            Rect::from_xywh(80.0, 90.0, 400.0, 140.0).unwrap(),
            sample["text"].as_str().unwrap(),
        );
        crate::tools::text::fit_frame(&mut annotation, None, &mut text.font_system);
        assert_state(&annotation, &sample["steps"][0], "initial fit");
        for (index, step) in sample["steps"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .skip(1)
        {
            let before = annotation.clone();
            let handle = if sample["name"] == "height-twice" {
                SelectionHandle::Bottom
            } else {
                SelectionHandle::Right
            };
            text_resize(
                &mut annotation,
                &before,
                handle,
                rect(&step["target"]),
                drag::Options {
                    bounds: before.frozen_bounds.unwrap(),
                    pixel_scale: 1.0,
                    keep_aspect: false,
                    snap_rotation: false,
                },
                &mut text,
            );
            assert_state(
                &annotation,
                &step["after"],
                &format!("{} {} step={index}", sample["name"], sample["text"]),
            );
        }
        let editor = crate::tools::text::ensure_text_editor(
            &annotation,
            &mut text.editors,
            &mut text.font_system,
        );
        let width = editor.with_buffer(|buffer| {
            (buffer.wrap() != cosmic_text::Wrap::None)
                .then_some(annotation.text_stored_bounds().unwrap().width())
        });
        crate::tools::text::fit_frame(&mut annotation, width, &mut text.font_system);
        assert_state(&annotation, &sample["fitted"], "final fit");
    }
}

/// 【文字存储测试】【原图与预览尺寸】六种比例保留126×45原图选框，预览尺寸分别与Qt实际字形测量相同，无参数、无返回值
#[test]
fn source_frame_and_painted_glyph_bounds_follow_their_own_scales() {
    for sample in reference()["scales"].as_array().unwrap() {
        let scale = sample["scale"].as_f64().unwrap() as f32;
        let (mut annotation, mut text) = prepared(
            Rect::from_xywh(80.0, 90.0, 400.0, 140.0).unwrap(),
            "Rotation",
        );
        crate::tools::text::fit_frame(&mut annotation, None, &mut text.font_system);
        annotation = annotation.scaled(scale);
        let editor = crate::tools::text::ensure_text_editor(
            &annotation,
            &mut text.editors,
            &mut text.font_system,
        );
        crate::tools::text::update_text_bbox_inline(&mut annotation, editor, &mut text.font_system);
        assert!((annotation.bbox.width() - 126.0 * scale).abs() < 0.0001);
        assert!((annotation.bbox.height() - 45.0 * scale).abs() < 0.0001);
        let painted = annotation.text_painted_bounds();
        assert!(
            (painted.width() - sample["after"]["painted"][0].as_f64().unwrap() as f32).abs()
                < 0.0001,
            "scale={scale} painted width"
        );
        assert!(
            (painted.height() - sample["after"]["painted"][1].as_f64().unwrap() as f32).abs()
                < 0.0001,
            "scale={scale} painted height"
        );
    }
}

/// 【文字存储测试】【空框定点限制】九个生产样本保留空矩形平移，首个定点限制后仍为文字和背景提供同一回退原点，无参数、无返回值
#[test]
fn empty_stored_rectangles_use_the_clamped_first_point_after_moving() {
    let reference = reference();
    let samples = reference["fallbacks"].as_array().unwrap();
    assert_eq!(samples.len(), 9);
    for (index, sample) in samples.iter().enumerate() {
        let (mut annotation, mut text) = prepared(rect(&sample["before"]["stored"]), "Rotation");
        let delta = (
            sample["delta"][0].as_f64().unwrap() as f32,
            sample["delta"][1].as_f64().unwrap() as f32,
        );
        annotation.translate_mut(delta.0, delta.1);
        transform::clamp_anchor(
            &mut annotation,
            Rect::from_xywh(0.0, 0.0, 640.0, 480.0).unwrap(),
            1.0,
        );
        let editor = crate::tools::text::ensure_text_editor(
            &annotation,
            &mut text.editors,
            &mut text.font_system,
        );
        crate::tools::text::update_text_bbox_inline(&mut annotation, editor, &mut text.font_system);
        assert_state(&annotation, &sample["after"], &format!("fallback={index}"));
        assert_eq!(
            annotation.text_painted_bounds().left(),
            annotation.bbox.left()
        );
        assert_eq!(
            annotation.text_painted_bounds().top(),
            annotation.bbox.top()
        );
        let mut natural = annotation.clone();
        if let AnnotationShape::Text { start, frame, .. } = &mut natural.shape {
            *start = (annotation.bbox.left(), annotation.bbox.top());
            frame.width = None;
            frame.height = None;
        }
        assert!(
            !annotation.same_edit_state(&natural),
            "empty dimensions must remain a real stored change"
        );
        let offset = (annotation.bbox.left() - 16.0, annotation.bbox.top() - 16.0);
        let mut actual = tiny_skia::Pixmap::new(256, 100).unwrap();
        let editor = crate::tools::text::ensure_text_editor(
            &annotation,
            &mut text.editors,
            &mut text.font_system,
        );
        crate::tools::text::render_text_annotation(
            &annotation,
            editor,
            &mut text.font_system,
            &mut text.swash_cache,
            &mut actual.as_mut(),
            offset,
            1.0,
            false,
        );
        natural.id = 10;
        let mut expected = tiny_skia::Pixmap::new(256, 100).unwrap();
        let editor = crate::tools::text::ensure_text_editor(
            &natural,
            &mut text.editors,
            &mut text.font_system,
        );
        crate::tools::text::render_text_annotation(
            &natural,
            editor,
            &mut text.font_system,
            &mut text.swash_cache,
            &mut expected.as_mut(),
            offset,
            1.0,
            false,
        );
        assert_eq!(
            actual.data(),
            expected.data(),
            "fallback={index}: glyph and background must share the first-point origin"
        );
    }
}
