//! 【表格滚动测试】【真实范围】逐项范围、末端偏移与小刻度累计对照实际 Qt 记录

use super::*;

/// 【表格滚动测试】【原版范围】按 state、rows 返回由原版列宽和视区构成的纯滚动范围，不替代列宽测量测试
pub(crate) fn ranges(state: &serde_json::Value, rows: usize) -> [Range; 2] {
    [
        Range::new(
            state["columns"]
                .as_array()
                .unwrap()
                .iter()
                .map(|value| value.as_f64().unwrap() as f32),
            state["viewport"][2].as_f64().unwrap() as f32,
        ),
        Range::new(
            std::iter::repeat_n(ROW_HEIGHT, rows),
            state["viewport"][3].as_f64().unwrap() as f32,
        ),
    ]
}

/// 【表格滚动测试】【实际末端】对照原版所有状态的索引范围、页项数和最终像素偏移，最后半行半列不越界，无参数、无返回值
#[test]
fn item_ranges_and_end_alignment_match_every_actual_qt_state() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/qt-ocr-table-scroll.json"
    ))
    .unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        for entry in case["states"].as_array().unwrap() {
            let state = &entry["state"];
            let ranges = ranges(state, if case["kind"] == "wide" { 3 } else { 40 });
            for (axis, name) in ["horizontal", "vertical"].into_iter().enumerate() {
                let expected = &state[name];
                if !expected["visible"].as_bool().unwrap() {
                    continue;
                }
                assert_eq!(
                    ranges[axis].maximum,
                    expected["maximum"].as_u64().unwrap() as usize,
                    "{} {} {name}",
                    case["kind"],
                    case["width"]
                );
                assert_eq!(
                    ranges[axis].page,
                    expected["page"].as_u64().unwrap() as usize
                );
                assert_eq!(
                    ranges[axis].offset(expected["value"].as_u64().unwrap() as usize),
                    state["offset"][axis].as_f64().unwrap() as f32,
                    "{} {} {} {name}",
                    case["kind"],
                    case["width"],
                    entry["name"]
                );
            }
        }
    }
}

/// 【表格滚动测试】【实际刻度序列】逐项比较原版完整刻度、小刻度累计、Ctrl 和 Shift；纯像素事件不伪造角度，无参数、无返回值
#[test]
fn wheel_accumulation_and_modified_pages_match_the_actual_qt_sequence() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/qt-ocr-table-scroll.json"
    ))
    .unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let ranges = ranges(
            &case["states"][0]["state"],
            if case["kind"] == "wide" { 3 } else { 40 },
        );
        let mut wheel = Wheel::default();
        let mut values = [0; 2];
        for entry in case["states"].as_array().unwrap().iter().skip(1) {
            let name = entry["name"].as_str().unwrap();
            let (angle, mods) = match name {
                "wheel-down" => ((0.0, -120.0), Mods::default()),
                "wheel-up" => ((0.0, 120.0), Mods::default()),
                "wheel-right" => {
                    values = [0; 2];
                    ((-120.0, 0.0), Mods::default())
                }
                "shift-wheel" => {
                    values = [0; 2];
                    (
                        (0.0, -120.0),
                        Mods {
                            shift: true,
                            ..Mods::default()
                        },
                    )
                }
                "ctrl-wheel" => {
                    values = [0; 2];
                    (
                        (0.0, -120.0),
                        Mods {
                            ctrl: true,
                            ..Mods::default()
                        },
                    )
                }
                "pixel-only" => {
                    values = [0; 2];
                    ((0.0, 0.0), Mods::default())
                }
                "partial-1" => {
                    values = [0; 2];
                    ((0.0, -20.0), Mods::default())
                }
                "partial-2" | "partial-3" | "partial-4" => ((0.0, -20.0), Mods::default()),
                _ => break,
            };
            let input = ScrollInput {
                angle,
                mods,
                ..ScrollInput::legacy((0.0, 0.0), (0.0, 15.0))
            };
            let steps = wheel.steps(input, [&ranges[0], &ranges[1]]);
            values = [
                ranges[0].moved(values[0], steps[0]),
                ranges[1].moved(values[1], steps[1]),
            ];
            for (axis, name) in ["horizontal", "vertical"].into_iter().enumerate() {
                assert_eq!(
                    values[axis],
                    entry["state"][name]["value"].as_u64().unwrap() as usize,
                    "{} {} {} {name}",
                    case["kind"],
                    case["width"],
                    entry["name"]
                );
            }
        }
    }
}

/// 【表格滚动测试】【边界安全】超大刻度、非数值、方向反转与空表格始终保持合法项数，不产生持续唤醒，无参数、无返回值
#[test]
fn huge_deltas_empty_ranges_and_direction_changes_are_bounded() {
    let range = Range::new(std::iter::repeat_n(28.0, 40), 313.0);
    let mut wheel = Wheel::default();
    let mut input = ScrollInput::legacy((0.0, 0.0), (0.0, 0.0));
    input.angle = (f64::NAN, f64::NEG_INFINITY);
    assert_eq!(wheel.steps(input, [&range, &range]), [0, 0]);
    input.angle = (0.0, -20.0);
    assert_eq!(wheel.steps(input, [&range, &range]), [0, 0]);
    input.angle = (0.0, 40.0);
    assert_eq!(wheel.steps(input, [&range, &range]), [0, -1]);
    input.angle = (0.0, -1e30);
    assert_eq!(wheel.steps(input, [&range, &range]), [0, 11]);
    let empty = Range::new(std::iter::empty(), 100.0);
    assert_eq!(empty.offset(usize::MAX), 0.0);
    assert_eq!(empty.index(1e30), 0);
    assert_eq!(empty.moved(0, i32::MAX), 0);
}
