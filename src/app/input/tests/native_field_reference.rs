//! 【字段输入测试】【原版状态】实际应用字段与 Qt 样本对照，包含 Return、实时字号与确认后颜色

use super::native_field_support::*;
use crate::{
    app::{panels, text_input},
    types::text_input::{Batch, Edit, Preedit, Replacement},
    ui::text_field::{LineEditState, validation::Rule},
};

/// 【字段输入测试】【编码定位】返回 text 中 UTF-16 units 对应的字节位置，不允许代理对中间位置
fn byte(text: &str, units: usize) -> usize {
    let mut used = 0;
    for (offset, ch) in text.char_indices() {
        if used == units {
            return offset;
        }
        used += ch.len_utf16();
    }
    assert_eq!(used, units);
    text.len()
}

/// 【字段输入测试】【事件转换】按 field 将 input 的 Qt 相对 UTF-16 替换转换为原生字节事件；返回完整事件
fn event(field: &LineEditState, input: &serde_json::Value) -> Edit {
    let preedit = input["preedit"].as_str().unwrap();
    let caret = input["caret"].as_i64().unwrap();
    let cursor = match caret {
        -1 => None,
        -2 => Some(preedit.len()),
        value => Some(byte(preedit, value as usize)),
    };
    let start = input["replace_start"].as_i64().unwrap();
    let length = input["replace_length"].as_u64().unwrap() as usize;
    let replacement = if start != 0 || length != 0 {
        let mut text = field.text.clone();
        let mut cursor = field.cursor_byte();
        if let Some((start, end)) = field.selection_byte_range() {
            text.replace_range(start..end, "");
            cursor = start;
        }
        let at = text[..cursor].encode_utf16().count() as i64 + start;
        let begin = byte(&text, at as usize);
        let end = byte(&text, at as usize + length);
        Some(Replacement {
            start: begin as isize - cursor as isize,
            length: end - begin,
        })
    } else {
        None
    };
    Edit {
        preedit: Preedit {
            text: preedit.into(),
            cursor: cursor.map(|at| (at, at)),
        },
        commit: input["commit"].as_str().unwrap().into(),
        replacement,
    }
}

/// 【字段输入测试】【应用状态】把 state 中正文、选择、历史、验证与业务值和 expected 对照，label 标识原版样本，无返回值
fn check(
    state: &crate::editor::EditorState,
    role: &str,
    expected: &serde_json::Value,
    label: &str,
) {
    let field = field(state);
    let units = |at| {
        field
            .text
            .chars()
            .take(at)
            .map(char::len_utf16)
            .sum::<usize>()
    };
    assert_eq!(
        field.text,
        expected["text"].as_str().unwrap(),
        "{label}: text"
    );
    assert_eq!(
        units(field.cursor),
        expected["cursor"].as_u64().unwrap() as usize,
        "{label}: cursor"
    );
    assert_eq!(
        units(field.selection_anchor.unwrap_or(field.cursor)),
        expected["anchor"].as_u64().unwrap() as usize,
        "{label}: anchor"
    );
    assert_eq!(
        field.selected_text().as_deref().unwrap_or(""),
        expected["selection"].as_str().unwrap(),
        "{label}: selection"
    );
    assert_eq!(
        field.can_undo(),
        expected["undo"].as_bool().unwrap(),
        "{label}: undo"
    );
    assert_eq!(
        field.can_redo(),
        expected["redo"].as_bool().unwrap(),
        "{label}: redo"
    );
    let rule = match role {
        "size" => Rule::Integer {
            min: 8,
            max: 300,
            suffix: " pt",
        },
        "hex" => Rule::Hex,
        _ => Rule::Text,
    };
    assert_eq!(
        rule.acceptable(&field.text),
        expected["acceptable"].as_bool().unwrap(),
        "{label}: acceptable"
    );
    if role == "size" {
        assert_eq!(
            crate::editor::text_edits::current_points(state),
            expected["points"].as_f64().unwrap() as f32,
            "{label}: points"
        );
    }
    if role == "hex" {
        let c = state.color_popover.sv_square.color().to_color_u8();
        let hex = format!(
            "#{:02x}{:02x}{:02x}{:02x}",
            c.alpha(),
            c.red(),
            c.green(),
            c.blue()
        );
        assert_eq!(
            hex,
            expected["color"].as_str().unwrap(),
            "{label}: confirmed color"
        );
    }
}

/// 【字段输入测试】【完整差分】逐个应用原版所有字段事件并对照宿主状态；协议无法表达的正向偏移仅经过实际字段分派
#[test]
fn qt_fields_match_application_content_history_and_business_values() {
    let rows: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/qt-line-edit-input-method.json"
    ))
    .unwrap();
    let mut compared = 0;
    for row in rows.as_array().unwrap() {
        let role = row["role"].as_str().unwrap();
        let initial = row["initial"].as_str().unwrap();
        let cursor = initial[..byte(
            initial,
            row["states"][0]["cursor"].as_u64().unwrap() as usize,
        )]
            .chars()
            .count();
        let anchor = initial[..byte(
            initial,
            row["states"][0]["anchor"].as_u64().unwrap() as usize,
        )]
            .chars()
            .count();
        let mut state = prepared(role, initial, cursor, anchor);
        let label = format!("{role}/{}", row["name"].as_str().unwrap());
        let states = row["states"].as_array().unwrap();
        check(&state, role, &states[0], &format!("{label} initial"));
        compared += 1;
        for (index, input) in row["events"].as_array().unwrap().iter().enumerate() {
            sync(&mut state);
            match input["kind"].as_str().unwrap() {
                "ime" => {
                    let event = event(field(&state), input);
                    match event.replacement {
                        Some(replace) if replace.start > 0 => {
                            assert!(text_input::fields::apply(&mut state, event, &mut 0));
                        }
                        _ => {
                            let (before, after) = event.replacement.map_or((0, 0), |replace| {
                                (
                                    (-replace.start) as u32,
                                    (replace.length as isize + replace.start) as u32,
                                )
                            });
                            deliver(
                                &mut state,
                                Batch {
                                    preedit: event.preedit,
                                    commit: event.commit,
                                    delete_before: before,
                                    delete_after: after,
                                },
                                true,
                            );
                        }
                    }
                }
                "undo" => super::key(&mut state, "ctrl+z", &mut 0),
                "redo" => super::key(&mut state, "ctrl+shift+z", &mut 0),
                "enter" => super::key(&mut state, "Enter", &mut 0),
                "set" => {
                    state.font_panel.fields.set_editing_text(
                        crate::ui::font_panel::state::Field::Search,
                        input["text"].as_str().unwrap().into(),
                    );
                    panels::text::keyboard::apply(&mut state, &mut 0);
                }
                "key" => {
                    let chord = format!(
                        "{}{}{}",
                        if input["ctrl"].as_bool().unwrap() {
                            "ctrl+"
                        } else {
                            ""
                        },
                        if input["shift"].as_bool().unwrap() {
                            "shift+"
                        } else {
                            ""
                        },
                        input["key"].as_str().unwrap()
                    );
                    super::key(&mut state, &chord, &mut 0);
                }
                _ => panic!("unknown original field event"),
            }
            check(
                &state,
                role,
                &states[index + 1],
                &format!("{label} event {index}"),
            );
            if input["kind"] != "enter" || role != "size" {
                assert_eq!(content(&state), "body", "{label}: annotation isolation");
            }
            compared += 1;
        }
    }
    assert_eq!(compared, 192);
}
