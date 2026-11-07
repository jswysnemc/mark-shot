//! 【区域识别回归】【交互查询】使用原版候选和操作验证完整包含链及指针容差

use super::helpers;
use crate::features::region_detection::{
    Cursor, Detector,
    candidate::merge,
    window::{Window, topmost},
};
use serde_json::Value;

/// 【区域识别回归】【动作对照】按 detector、input、expected 逐项执行真实动作，返回值、当前框和手动身份必须相同
pub(super) fn actions(detector: &Detector, input: &Value, expected: &Value) {
    let name = input["name"].as_str().unwrap();
    let mut cursor = Cursor::default();
    for (index, action) in input["actions"].as_array().unwrap().iter().enumerate() {
        let point = if let Some(value) = action.get("normalized") {
            detector.point_from_normalized(
                value[0].as_f64().unwrap() as f32,
                value[1].as_f64().unwrap() as f32,
            )
        } else {
            (
                action["point"][0].as_i64().unwrap() as i32,
                action["point"][1].as_i64().unwrap() as i32,
            )
        };
        let selection = match action["kind"].as_str().unwrap() {
            "region" => helpers::encoded(detector.region_at(point)),
            "regions" => Value::Array(
                detector
                    .regions_at(point)
                    .into_iter()
                    .map(helpers::encoded)
                    .collect(),
            ),
            "hover" => helpers::encoded(cursor.hover(detector, point)),
            "step" => helpers::encoded(cursor.step(
                detector,
                point,
                action["direction"].as_i64().unwrap() as i32,
            )),
            "reset" => {
                cursor.reset();
                Value::Null
            }
            _ => panic!("unknown region reference action"),
        };
        let expected = &expected["actions"][index];
        assert_eq!(
            serde_json::json!([point.0, point.1]),
            expected["point"],
            "{name}: point {index}"
        );
        assert_eq!(
            selection, expected["selection"],
            "{name}: selection {index}"
        );
        assert_eq!(
            cursor.manual_level(),
            expected["manual"].as_bool().unwrap(),
            "{name}: manual {index}"
        );
        assert_eq!(
            cursor
                .current()
                .map(helpers::encoded)
                .unwrap_or(Value::Null),
            expected["current"],
            "{name}: current {index}"
        );
    }
}

/// 【区域识别回归】【候选场景】验证所有原版固定候选、窗口约束、合并与滚轮序列，无参数、无返回值
#[test]
fn manual_candidates_and_cursor_actions_match_qt() {
    let (inputs, expected) = helpers::fixture();
    for input in inputs.iter().filter(|input| input["mode"] == "manual") {
        let name = input["name"].as_str().unwrap();
        let mut candidates = input["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .map(helpers::candidate)
            .collect();
        if let Some(merges) = input["merges"].as_array() {
            for value in merges {
                merge(
                    &mut candidates,
                    helpers::candidate(&value["candidate"]),
                    value["tolerance"].as_i64().unwrap() as i32,
                );
            }
        }
        let mut detector = Detector::from_candidates(
            input["width"].as_i64().unwrap() as i32,
            input["height"].as_i64().unwrap() as i32,
            candidates,
        );
        let windows: Vec<_> = input["window_rects"]
            .as_array()
            .unwrap()
            .iter()
            .map(helpers::rect)
            .collect();
        detector.add_windows(&windows);
        helpers::candidates(detector.candidates(), &expected[name]["candidates"], name);
        actions(&detector, input, &expected[name]);
    }
}

/// 【窗口识别回归】【真实层级】验证负层级、缺失层级、最低整数层级与边界命中，无参数、无返回值
#[test]
fn window_z_order_and_fallback_match_qt() {
    let (inputs, expected) = helpers::fixture();
    for input in inputs.iter().filter(|input| input["mode"] == "windows") {
        let name = input["name"].as_str().unwrap();
        let windows: Vec<_> = input["windows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|window| Window {
                rect: helpers::rect(&window["rect"]),
                z_order: window["z"].as_i64().map(|value| value as i32),
            })
            .collect();
        for (index, action) in input["actions"].as_array().unwrap().iter().enumerate() {
            let point = (
                action["point"][0].as_i64().unwrap() as i32,
                action["point"][1].as_i64().unwrap() as i32,
            );
            assert_eq!(
                topmost(&windows, point)
                    .map(helpers::encoded)
                    .unwrap_or(Value::Null),
                expected[name]["actions"][index],
                "{name}: {index}"
            );
        }
    }
}
