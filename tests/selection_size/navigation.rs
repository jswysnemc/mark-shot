//! 【选区尺寸测试】【词语对照】用原版键盘样本验证边界、选择方向、拒绝删除及撤销恢复

use mark_shot::ui::{
    selection_size::{field::NumberField, navigation},
    text_field::LineEditState,
};
use serde_json::Value;

/// 【选区尺寸测试】【状态恢复】按 before 返回相同正文、光标和选择方向的活动字段，不模拟按键结果
fn field(before: &Value) -> NumberField {
    let text = before["text"].as_str().unwrap().to_owned();
    let mut field = NumberField::new(12345, 10000000);
    field.begin();
    field.editor = LineEditState::new(text);
    field.editor.cursor = before["cursor"].as_u64().unwrap() as usize;
    let start = before["selection_start"].as_i64().unwrap();
    if start >= 0 {
        let start = start as usize;
        let end = start + before["selected"].as_str().unwrap().chars().count();
        field.editor.selection_anchor = Some(if field.editor.cursor == start {
            end
        } else {
            start
        });
    }
    field
}

/// 【选区尺寸测试】【完整字段】按 expected 核对 editor 正文、光标、选择与历史，sample 标识原版输入
fn equal(editor: &LineEditState, expected: &Value, sample: &Value) {
    assert_eq!(editor.text, expected["text"].as_str().unwrap(), "{sample}");
    assert_eq!(
        editor.cursor,
        expected["cursor"].as_u64().unwrap() as usize,
        "{sample}"
    );
    assert_eq!(
        editor.selected_text().unwrap_or_default(),
        expected["selected"].as_str().unwrap(),
        "{sample}"
    );
    assert_eq!(
        editor
            .selection_range()
            .map_or(-1, |(start, _)| start as i64),
        expected["selection_start"].as_i64().unwrap(),
        "{sample}"
    );
    assert_eq!(
        editor.can_undo(),
        expected["undo"].as_bool().unwrap(),
        "{sample}"
    );
    assert_eq!(
        editor.can_redo(),
        expected["redo"].as_bool().unwrap(),
        "{sample}"
    );
}

/// 【选区尺寸测试】【生产词语键】所有原版词语移动、选择、删除和撤销状态严格相等
#[test]
fn words_and_rejected_deletion_match_production_fields() {
    let reference: Value = serde_json::from_str(include_str!(
        "../fixtures/qt-selection-size/field-keys.json"
    ))
    .unwrap();
    let mut count = 0;
    for sample in reference["samples"].as_array().unwrap() {
        let name = sample["name"].as_str().unwrap();
        let mut field = field(&sample["before"]);
        match name {
            "word_left" => navigation::move_word(&mut field, true, false),
            "word_right" => navigation::move_word(&mut field, false, false),
            "select_word_left" => navigation::move_word(&mut field, true, true),
            "select_word_right" => navigation::move_word(&mut field, false, true),
            "delete_word_left" => navigation::delete_word(&mut field, true),
            "ctrl_shift_backspace" => navigation::extend_delete_backward(&mut field),
            "delete_word_right" => navigation::delete_word(&mut field, false),
            _ => continue,
        }
        count += 1;
        equal(&field.editor, &sample["after"], sample);
        if let Some(undone) = sample.get("undone") {
            field.edit(|editor| {
                editor.undo();
            });
            equal(&field.editor, undone, sample);
        }
    }
    assert_eq!(count, 175);
}
