//! 【旋转文字对照】【实际排版】原图选框与预览字形分别核对生产Qt尺寸，存储宽高独立于实际字形

use super::*;
use cosmic_text::Edit;
use std::collections::HashMap;

/// 【旋转文字对照】【生产样本】返回完整原版字体与测量函数的结果，不由当前实现生成预期，无参数
fn reference() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../fixtures/qt-annotation-rotation/text-reference.json"
    ))
    .unwrap()
}

/// 【旋转文字对照】【自然选框】162个宽度、高度及比例样本通过实际字体排版核对，容许不同字形引擎的一像素取整差异，无参数、无返回值
#[test]
fn visible_text_bounds_match_production_qt_natural_measurement() {
    let reference = reference();
    let samples = reference["samples"].as_array().unwrap();
    assert_eq!(samples.len(), 162);
    let mut fonts = mark_shot::fonts::bundled();
    for sample in samples {
        let scale = sample["scale"].as_f64().unwrap() as f32;
        let mut annotation = annotation(AnnotationShape::Text {
            frame: mark_shot::features::text::TextFrame {
                width: Some(sample["stored_width"].as_f64().unwrap() as f32 * scale),
                height: Some(sample["stored_height"].as_f64().unwrap() as f32 * scale),
                ..Default::default()
            },
            appearance: mark_shot::features::text::TextAppearance {
                family: sample["family"].as_str().unwrap().into(),
                size_scale: scale,
                ..Default::default()
            },
            start: (80.0 * scale, 90.0 * scale),
            content: sample["text"].as_str().unwrap().into(),
            font_size: 20.0,
            bold: true,
            italic: false,
            outline: None,
        });
        let mut editors = HashMap::new();
        let editor =
            mark_shot::tools::text::ensure_text_editor(&annotation, &mut editors, &mut fonts);
        mark_shot::tools::text::update_text_bbox_inline(&mut annotation, editor, &mut fonts);
        let wraps = editor.with_buffer(|buffer| buffer.wrap() != cosmic_text::Wrap::None);
        assert_eq!(wraps, sample["wraps"].as_bool().unwrap(), "{sample}");
        let painted = annotation.text_painted_bounds();
        for (index, actual) in [painted.width(), painted.height()].into_iter().enumerate() {
            let expected = sample["size"][index].as_f64().unwrap();
            assert!(
                (f64::from(actual) - expected).abs() <= 1.0,
                "actual={actual} expected={expected} sample={sample}"
            );
        }
        let source = samples
            .iter()
            .find(|source| {
                source["scale"] == 1.0
                    && source["family"] == sample["family"]
                    && source["text"] == sample["text"]
                    && source["stored_width"] == sample["stored_width"]
                    && source["stored_height"] == sample["stored_height"]
            })
            .unwrap();
        for (index, actual) in [annotation.bbox.width(), annotation.bbox.height()]
            .into_iter()
            .enumerate()
        {
            let expected = source["size"][index].as_f64().unwrap() * f64::from(scale);
            assert!(
                (f64::from(actual) - expected).abs() <= f64::from(scale),
                "source actual={actual} expected={expected} sample={sample}"
            );
        }
    }
}
