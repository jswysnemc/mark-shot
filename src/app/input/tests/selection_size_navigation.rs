//! 【选区尺寸测试】【键盘分派】真实应用入口对照原版字段按键，剪切复制另在私有 Wayland 验证

use super::*;
use serde_json::Value;

/// 【选区尺寸测试】【原版键名】按 name 返回等价应用按键，未知样本拒绝处理
fn chord(name: &str) -> &'static str {
    match name {
        "left" => "Left",
        "right" => "Right",
        "word_left" => "Ctrl+Left",
        "word_right" => "Ctrl+Right",
        "select_word_left" => "Ctrl+Shift+Left",
        "select_word_right" => "Ctrl+Shift+Right",
        "delete_word_left" => "Ctrl+Backspace",
        "delete_word_right" => "Ctrl+Delete",
        "alt_left" => "Alt+Left",
        "alt_right" => "Alt+Right",
        "ctrl_alt_left" => "Ctrl+Alt+Left",
        "ctrl_alt_right" => "Ctrl+Alt+Right",
        "ctrl_shift_backspace" => "Ctrl+Shift+Backspace",
        "ctrl_shift_delete" => "Ctrl+Shift+Delete",
        "home" => "Home",
        "end" => "End",
        "ctrl_home" => "Ctrl+Home",
        "ctrl_end" => "Ctrl+End",
        "select_all" => "Ctrl+A",
        "ctrl_shift_a" => "Ctrl+Shift+A",
        "ctrl_alt_a" => "Ctrl+Alt+A",
        _ => panic!("unknown production key sample: {name}"),
    }
}

/// 【选区尺寸测试】【字段恢复】按 before 初始化 state 活动宽度字段，正文、光标和锚点来自原版输入前状态，无返回值
fn restore(state: &mut EditorState, before: &Value) {
    let field = &mut state.selection_size.panel.as_mut().unwrap().fields[0];
    field.maximum = 10000000;
    field.editing = true;
    field.editor =
        crate::ui::text_field::LineEditState::new(before["text"].as_str().unwrap().to_owned());
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
}

/// 【选区尺寸测试】【字段结果】按 sample 的 after 或 undone 核对 state 宽度字段，包含光标、选择及历史
fn equal(state: &EditorState, expected: &Value, sample: &Value) {
    let editor = &state.selection_size.panel.as_ref().unwrap().fields[0].editor;
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

/// 【选区尺寸测试】【标准按键】525 个原版按键状态通过应用输入入口严格对照，不依赖单独调用导航函数
#[test]
fn production_keyboard_samples_match_application_dispatch() {
    let reference: Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/qt-selection-size/field-keys.json"
    ))
    .unwrap();
    let mut state = super::selection_size::selected();
    key(&mut state, "Ctrl+R", &mut 0);
    let mut count = 0;
    for sample in reference["samples"].as_array().unwrap() {
        let name = sample["name"].as_str().unwrap();
        if name == "shift_delete" {
            continue;
        }
        restore(&mut state, &sample["before"]);
        key(&mut state, chord(name), &mut 0);
        equal(&state, &sample["after"], sample);
        if let Some(undone) = sample.get("undone") {
            key(&mut state, "Ctrl+Z", &mut 0);
            equal(&state, undone, sample);
        }
        assert_eq!(state.selection.zone.unwrap().width(), 300.0);
        assert!(!state.cancel && state.finish.is_none());
        count += 1;
    }
    assert_eq!(count, 525);
}
