//! 【滚动条区域测试】【原版几何】对照真实样式控件区域及滑块的可逆索引

use super::*;

/// 【滚动条区域测试】【完整状态】比较原版所有可见滚动条的滑块、轨道、箭头及两段翻页区，无参数、无返回值
#[test]
fn visible_bar_parts_match_actual_qt_subcontrols_in_every_state() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/qt-ocr-table-scroll.json"
    ))
    .unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        for entry in case["states"].as_array().unwrap() {
            let state = &entry["state"];
            let ranges = super::super::table_scroll_model::tests::ranges(
                state,
                if case["kind"] == "wide" { 3 } else { 40 },
            );
            for (index, (axis, name)) in [
                (Axis::Horizontal, "horizontal"),
                (Axis::Vertical, "vertical"),
            ]
            .into_iter()
            .enumerate()
            {
                let expected = &state[name];
                if !expected["visible"].as_bool().unwrap() {
                    continue;
                }
                let rect = &expected["rect"];
                let bar = Bar::new(
                    axis,
                    Rect::from_xywh(
                        0.0,
                        0.0,
                        rect[2].as_f64().unwrap() as f32,
                        rect[3].as_f64().unwrap() as f32,
                    )
                    .unwrap(),
                    &ranges[index],
                    expected["value"].as_u64().unwrap() as usize,
                );
                for (key, actual) in [
                    ("slider", Some(bar.slider)),
                    ("groove", Some(bar.groove)),
                    ("sub_line", bar.previous_line),
                    ("add_line", bar.next_line),
                    ("sub_page", bar.previous_page),
                    ("add_page", bar.next_page),
                ] {
                    let wanted = &expected["controls"][key];
                    if wanted[2] == 0 || wanted[3] == 0 {
                        assert!(actual.is_none(), "{name} {key}");
                        continue;
                    }
                    let actual = actual.unwrap();
                    assert_eq!(
                        serde_json::json!([
                            actual.x(),
                            actual.y(),
                            actual.width(),
                            actual.height()
                        ]),
                        serde_json::json!([
                            wanted[0].as_f64().unwrap(),
                            wanted[1].as_f64().unwrap(),
                            wanted[2].as_f64().unwrap(),
                            wanted[3].as_f64().unwrap()
                        ]),
                        "{} {} {} {name} {key}",
                        case["kind"],
                        case["width"],
                        entry["name"]
                    );
                }
            }
        }
    }
}

/// 【滚动条区域测试】【拖动锚点】滑块按下保持当前位置，极端坐标只到合法边缘，无参数、无返回值
#[test]
fn dragging_retains_the_press_anchor_and_clamps_to_track_ends() {
    let range = Range::new(std::iter::repeat_n(28.0, 40), 313.0);
    for axis in [Axis::Horizontal, Axis::Vertical] {
        let bounds = match axis {
            Axis::Horizontal => Rect::from_xywh(20.0, 30.0, 343.0, 14.0),
            Axis::Vertical => Rect::from_xywh(20.0, 30.0, 6.0, 343.0),
        }
        .unwrap();
        for value in 0..=range.maximum {
            let bar = Bar::new(axis, bounds, &range, value);
            let (start, length) = axis.geometry(bar.slider);
            assert_eq!(bar.value_at(start + length / 2.0, length / 2.0), value);
            assert_eq!(bar.value_at(-10000.0, length / 2.0), 0);
            assert_eq!(bar.value_at(10000.0, length / 2.0), range.maximum);
        }
    }
}
