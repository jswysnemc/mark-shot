//! 【字段输入法测试】【原版模型】通过实际 Qt 样本核验单行正文和局部撤销，宿主确认动作另由生产路由验证

use mark_shot::{
    types::text_input::{Edit, Preedit, Replacement},
    ui::text_field::{LineEditState, validation::Rule},
};

/// 【字段输入法测试】【UTF-16 转换】返回 text 中 units 对应的 UTF-8 字节，代理对中间位置返回空值
fn byte_at_utf16(text: &str, units: usize) -> Option<usize> {
    let mut consumed = 0;
    for (byte, character) in text.char_indices() {
        if consumed == units {
            return Some(byte);
        }
        consumed += character.len_utf16();
        if consumed > units {
            return None;
        }
    }
    (consumed == units).then_some(text.len())
}

/// 【字段输入法测试】【字符光标】将 field 的字符 index 转换为 UTF-16 位置，返回数值，不假定汉字字节长度
fn units(field: &LineEditState, index: usize) -> usize {
    field.text.chars().take(index).map(char::len_utf16).sum()
}

/// 【字段输入法测试】【事件适配】按 field 与 rule 将 event 的实际 Qt UTF-16 替换映射为字节事件，返回生产参数
fn event(field: &LineEditState, _rule: Rule, event: &serde_json::Value) -> Edit {
    let preedit = event["preedit"].as_str().unwrap();
    let commit = event["commit"].as_str().unwrap();
    let caret = event["caret"].as_i64().unwrap();
    let cursor = match caret {
        -1 => None,
        -2 => Some(preedit.len()),
        value => Some(byte_at_utf16(preedit, value as usize).unwrap()),
    };
    let replacing = event["replace_start"] != 0 || event["replace_length"] != 0;
    let replacement = if replacing {
        let mut text = field.text.clone();
        let mut cursor = field.cursor_byte();
        if let Some((start, end)) = field.selection_byte_range() {
            text.replace_range(start..end, "");
            cursor = start;
        }
        let cursor_units = text[..cursor].encode_utf16().count() as i64;
        let start_units = cursor_units + event["replace_start"].as_i64().unwrap();
        let end_units = start_units + event["replace_length"].as_i64().unwrap();
        let start = byte_at_utf16(&text, start_units as usize).unwrap();
        let end = byte_at_utf16(&text, end_units as usize).unwrap();
        Some(Replacement {
            start: start as isize - cursor as isize,
            length: end - start,
        })
    } else {
        None
    };
    Edit {
        preedit: Preedit {
            text: preedit.into(),
            cursor: cursor.map(|value| (value, value)),
        },
        commit: commit.into(),
        replacement,
    }
}

/// 【字段输入法测试】【实际状态】将 field 和 rule 的生产正文、光标、验证结果及撤销能力与 expected 比较，无返回值
fn check(field: &LineEditState, rule: Rule, expected: &serde_json::Value, label: &str) {
    assert_eq!(
        field.text,
        expected["text"].as_str().unwrap(),
        "{label}: text"
    );
    assert_eq!(
        units(field, field.cursor),
        expected["cursor"].as_u64().unwrap() as usize,
        "{label}: cursor"
    );
    assert_eq!(
        units(field, field.selection_anchor.unwrap_or(field.cursor)),
        expected["anchor"].as_u64().unwrap() as usize,
        "{label}: anchor"
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
    assert_eq!(
        rule.acceptable(&field.text),
        expected["acceptable"].as_bool().unwrap(),
        "{label}: acceptable"
    );
}

/// 【字段输入法测试】【状态差分】原版事件通过单行生产模型执行；不把宿主 Return、字号业务或颜色确认当作模型测试通过
#[test]
fn qt_non_host_events_match_single_line_content_selection_and_history() {
    let rows: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/qt-line-edit-input-method.json")).unwrap();
    let mut compared = 0;
    for row in rows.as_array().unwrap() {
        if row["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|input| input["kind"] == "key")
        {
            continue;
        }
        let role = row["role"].as_str().unwrap();
        let rule = match role {
            "size" => Rule::Integer {
                min: 8,
                max: 300,
                suffix: " pt",
            },
            "hex" => Rule::Hex,
            _ => Rule::Text,
        };
        let initial = row["initial"].as_str().unwrap();
        let mut field = LineEditState::new(initial.into());
        let cursor = byte_at_utf16(initial, row["cursor"].as_u64().unwrap() as usize).unwrap();
        let anchor = byte_at_utf16(initial, row["anchor"].as_u64().unwrap() as usize).unwrap();
        field.cursor = initial[..cursor].chars().count();
        field.selection_anchor = (cursor != anchor).then(|| initial[..anchor].chars().count());
        let states = row["states"].as_array().unwrap();
        check(
            &field,
            rule,
            &states[0],
            &format!("{role}/{} initial", row["name"]),
        );
        compared += 1;
        for (index, input) in row["events"].as_array().unwrap().iter().enumerate() {
            match input["kind"].as_str().unwrap() {
                "ime" => {
                    field.input_method(event(&field, rule, input), rule);
                }
                "undo" => {
                    field.undo();
                }
                "redo" => {
                    field.redo();
                }
                "set" => {
                    field = LineEditState::new(input["text"].as_str().unwrap().into());
                }
                "enter" if role == "size" => break,
                "enter" => continue,
                _ => panic!("unsupported actual Qt fixture event"),
            }
            check(
                &field,
                rule,
                &states[index + 1],
                &format!("{role}/{} event {index}", row["name"]),
            );
            compared += 1;
        }
    }
    assert!(compared >= 100);
}
