//! 【区域识别回归】【数据读取】读取原版采集输入与输出，保持独立期望来源

use crate::features::region_detection::{Candidate, PixelRect};
use serde_json::Value;
use std::{collections::HashMap, sync::OnceLock};

/// 【区域识别回归】【参考数据】无参数，返回一次解析的输入场景和按名称索引的实际 Qt 输出
pub(super) fn fixture() -> &'static (Vec<Value>, HashMap<String, Value>) {
    static DATA: OnceLock<(Vec<Value>, HashMap<String, Value>)> = OnceLock::new();
    DATA.get_or_init(|| {
        let mut inputs = Vec::new();
        let mut outputs = HashMap::new();
        for (input, expected) in [
            (
                include_str!(
                    "../../../../tests/fixtures/qt-region-detection-data-r1/manual-input.json"
                ),
                include_str!(
                    "../../../../tests/fixtures/qt-region-detection-data-r1/manual-expected.json"
                ),
            ),
            (
                include_str!(
                    "../../../../tests/fixtures/qt-region-detection-data-r1/image-input.json"
                ),
                include_str!(
                    "../../../../tests/fixtures/qt-region-detection-data-r1/image-expected.json"
                ),
            ),
            (
                include_str!(
                    "../../../../tests/fixtures/qt-region-detection-data-r1/windows-input.json"
                ),
                include_str!(
                    "../../../../tests/fixtures/qt-region-detection-data-r1/windows-expected.json"
                ),
            ),
        ] {
            let input: Value = serde_json::from_str(input).unwrap();
            let expected: Value = serde_json::from_str(expected).unwrap();
            inputs.extend(input["cases"].as_array().unwrap().iter().cloned());
            outputs.extend(
                expected["cases"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|case| (case["name"].as_str().unwrap().to_owned(), case.clone())),
            );
        }
        (inputs, outputs)
    })
}

/// 【区域识别回归】【矩形读取】按 value 的 x、y、width、height 构造矩形，返回整数像素范围
pub(super) fn rect(value: &Value) -> PixelRect {
    let a = value.as_array().unwrap();
    PixelRect::new(
        a[0].as_i64().unwrap() as i32,
        a[1].as_i64().unwrap() as i32,
        a[2].as_i64().unwrap() as i32,
        a[3].as_i64().unwrap() as i32,
    )
}

/// 【区域识别回归】【矩形输出】按 value 返回与原版相同的四项整数数组
pub(super) fn encoded(value: PixelRect) -> Value {
    serde_json::json!([value.left, value.top, value.width(), value.height()])
}

/// 【区域识别回归】【候选读取】按 value 读取固定候选，返回原始质量、边界和来源
pub(super) fn candidate(value: &Value) -> Candidate {
    Candidate {
        rect: rect(&value["rect"]),
        quality: value["quality"].as_f64().unwrap() as f32,
        sides: std::array::from_fn(|index| value["sides"][index].as_f64().unwrap() as f32),
        interior_edges: value["interior_edges"].as_f64().unwrap() as f32,
        sources: serde_json::from_value(value["sources"].clone()).unwrap(),
    }
}

/// 【区域识别回归】【候选比较】比较 actual 与 expected 中全部边界、来源和评分，name 标识失败场景，无返回值
pub(super) fn candidates(actual: &[Candidate], expected: &Value, name: &str) {
    let expected = expected.as_array().unwrap();
    assert_eq!(actual.len(), expected.len(), "{name}: candidate count");
    let mut used = vec![false; expected.len()];
    for value in actual {
        let index = expected
            .iter()
            .enumerate()
            .find_map(|(index, item)| {
                (!used[index] && value.rect == rect(&item["rect"])).then_some(index)
            })
            .unwrap_or_else(|| panic!("{name}: unexpected candidate {:?}", value.rect));
        used[index] = true;
        let source = candidate(&expected[index]);
        assert!(
            (value.quality - source.quality).abs() < 0.000_002,
            "{name}: quality {:?}",
            value.rect
        );
        assert!(
            (value.interior_edges - source.interior_edges).abs() < 0.000_002,
            "{name}: density {:?}",
            value.rect
        );
        for index in 0..4 {
            assert!(
                (value.sides[index] - source.sides[index]).abs() < 0.000_002,
                "{name}: side {index}"
            );
        }
        assert_eq!(
            value.sources.len(),
            source.sources.len(),
            "{name}: source count"
        );
        assert!(
            value
                .sources
                .iter()
                .all(|item| source.sources.contains(item)),
            "{name}: sources {:?}",
            value.rect
        );
    }
}

/// 【区域识别回归】【像素摘要】按 bytes 返回与独立参考相同的六十四位 FNV 摘要
pub(super) fn hash(bytes: &[u8]) -> String {
    let hash = bytes
        .iter()
        .fold(14_695_981_039_346_656_037u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(1_099_511_628_211)
        });
    format!("{hash:x}")
}
