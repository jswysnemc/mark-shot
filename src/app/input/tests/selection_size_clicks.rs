//! 【选区尺寸测试】【平台点击】从应用入口重放 Qt 平台实际生成的位移多击与右键插入，不手工指定双击类型

use super::*;
use crate::{features::selection_size::ratio::Field, types::MouseButton};
use serde_json::Value;

/// 【选区尺寸测试】【点击参考】返回生产 Qt 平台与字段生成的完整序列，不使用当前点击算法构造预期
fn reference() -> Value {
    serde_json::from_str(include_str!(
        "../../../../tests/fixtures/qt-selection-size/field-clicks.json"
    ))
    .unwrap()
}

/// 【选区尺寸测试】【原版字段】按 text 返回实际宽度12345的截图字段，原图、输出和缓存尺寸一致，不用孤立数值绕过应用同步
fn editing(text: &str) -> EditorState {
    let mut state = editor();
    // 1. 【选区尺寸测试】【真实图像】12800×160原图可容纳五位数宽度，缓存与逻辑输出使用相同尺寸
    let mut image = Pixmap::new(12800, 160).unwrap();
    image.fill(Color::from_rgba8(30, 34, 38, 255));
    state.captures[0].pixmap = image.clone().into();
    state.canvas[0] = image.clone();
    state.dimmed[0] = image;
    state.annotations_layer[0] = Pixmap::new(12800, 160).unwrap();
    state.placements[0].size = (12800, 160);
    state.selection.zone = tiny_skia::Rect::from_xywh(100.0, 70.0, 12345.0, 18.0);
    super::super::refresh_panels(&mut state, &mut 0);
    let field = &mut state.selection_size.panel.as_mut().unwrap().fields[0];
    field.begin();
    field.editor = crate::ui::text_field::LineEditState::new(text.into());
    state
}

/// 【选区尺寸测试】【实际按钮】按 step 的字段坐标投递 state 完整按钮序列，无返回值，不指定双击或三击结果
fn apply(state: &mut EditorState, step: &Value) {
    let wait = step["wait_ms"].as_u64().unwrap();
    if wait > 0 {
        std::thread::sleep(std::time::Duration::from_millis(wait));
    }
    let rect = state
        .selection_size
        .panel
        .as_ref()
        .unwrap()
        .field_rect(Field::Width)
        .unwrap();
    let point = &step["point"];
    pointer(
        state,
        f64::from(rect.left()) + point[0].as_f64().unwrap(),
        f64::from(rect.top()) + point[1].as_f64().unwrap(),
        &mut 0,
    );
    let button = match step["action"].as_str().unwrap() {
        "left" => MouseButton::Left,
        "right" => MouseButton::Right,
        "up" => {
            key(state, "Up", &mut 0);
            return;
        }
        "focus_out" => {
            crate::app::event_loop::handle_event(
                state,
                crate::types::OverlayEvent::FocusLost { monitor_idx: 0 },
                &mut 0,
            );
            return;
        }
        "focus_in" => {
            crate::app::event_loop::handle_event(
                state,
                crate::types::OverlayEvent::Focus { monitor_idx: 0 },
                &mut 0,
            );
            key(state, "Ctrl+R", &mut 0);
            return;
        }
        _ => return,
    };
    for pressed in [true, false] {
        crate::app::event_loop::handle_event(
            state,
            crate::types::OverlayEvent::PointerButton {
                button,
                pressed,
                serial: None,
                timestamp: Some(
                    10000 + step["time_ms"].as_u64().unwrap() as u32 + u32::from(!pressed),
                ),
            },
            &mut 0,
        );
        assert!(state.input.pointer_timestamp.is_none());
    }
}

/// 【选区尺寸测试】【完整状态】按 sample 与 step 核对 state 字段正文、游标和选择，返回空值
fn equal(state: &EditorState, step: &Value, sample: &Value) {
    let field = &state.selection_size.panel.as_ref().unwrap().fields[0];
    let expected = &step["after"];
    assert_eq!(
        field.editor.text,
        expected["text"].as_str().unwrap(),
        "{sample} step={step}"
    );
    assert_eq!(
        field.editing,
        step["editing"].as_bool().unwrap(),
        "{sample} step={step}"
    );
    assert_eq!(
        field.editor.cursor,
        expected["cursor"].as_u64().unwrap() as usize,
        "{sample} step={step}"
    );
    assert_eq!(
        field.editor.selected_text().unwrap_or_default(),
        expected["selected"].as_str().unwrap(),
        "{sample} step={step}"
    );
}

/// 【选区尺寸测试】【位移三击】原版使用十像素曼哈顿距离；五个正文和十个位移序列逐步相等
#[test]
fn shifted_triple_clicks_match_production_platform_sequences() {
    let reference = reference();
    let mut count = 0;
    for sample in reference["samples"].as_array().unwrap() {
        if sample["name"] != "triple_distance" {
            continue;
        }
        let mut state = editing(sample["text"].as_str().unwrap());
        for step in sample["steps"].as_array().unwrap() {
            apply(&mut state, step);
            equal(&state, step, sample);
        }
        count += 1;
    }
    assert_eq!(count, 50);
}

/// 【选区尺寸测试】【右键交错】右键结束平台双击序列，但保留原字段三击计时器，五个原版正文逐步相等
#[test]
fn right_click_preserves_the_production_triple_click_timer() {
    let reference = reference();
    let mut count = 0;
    for sample in reference["samples"].as_array().unwrap() {
        if sample["name"] != "double_then_right" {
            continue;
        }
        let mut state = editing(sample["text"].as_str().unwrap());
        for step in sample["steps"].as_array().unwrap() {
            apply(&mut state, step);
            equal(&state, step, sample);
        }
        count += 1;
    }
    assert_eq!(count, 5);
}

/// 【选区尺寸测试】【全部平台序列】200组原版平台序列验证门限、原始时间、移动、右键、分数命中及真实失焦提交和重开
#[test]
fn all_production_pointer_sequences_match_application_and_field_entry_points() {
    let reference = reference();
    let mut count = 0;
    for sample in reference["samples"].as_array().unwrap() {
        let mut state = editing(sample["text"].as_str().unwrap());
        for step in sample["steps"].as_array().unwrap() {
            apply(&mut state, step);
            equal(&state, step, sample);
        }
        count += 1;
    }
    assert_eq!(count, 200);
}

/// 【选区尺寸测试】【合并移出返回】应用入口收到最终原位与中途极值时保持单击，不能再次选择数字段
#[test]
fn coalesced_excursion_invalidates_double_click_at_application_dispatch() {
    use crate::types::{OverlayEvent, pointer_motion::PointerMotion};
    let mut state = editing("+12345");
    let rect = state
        .selection_size
        .panel
        .as_ref()
        .unwrap()
        .field_rect(Field::Width)
        .unwrap();
    let point = (f64::from(rect.right()) - 8.0, f64::from(rect.top()) + 10.0);
    pointer(&mut state, point.0, point.1, &mut 0);
    for timestamp in [10000, 10042] {
        if timestamp == 10042 {
            crate::app::event_loop::handle_event(
                &mut state,
                OverlayEvent::PointerMove {
                    monitor_idx: 0,
                    x: point.0,
                    y: point.1,
                    history: vec![PointerMotion {
                        monitor_idx: 0,
                        min: (point.0 - 15.0, point.1),
                        max: point,
                    }],
                },
                &mut 0,
            );
        }
        for pressed in [true, false] {
            crate::app::event_loop::handle_event(
                &mut state,
                OverlayEvent::PointerButton {
                    button: MouseButton::Left,
                    pressed,
                    serial: None,
                    timestamp: Some(timestamp),
                },
                &mut 0,
            );
        }
    }
    let field = &state.selection_size.panel.as_ref().unwrap().fields[0];
    assert_eq!(field.editor.text, "+12345");
    assert!(field.editor.selected_text().is_none());
}
