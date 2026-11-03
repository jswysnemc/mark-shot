use cosmic_text::FontSystem;
use mark_shot::{
    backend::wayland::window::KeyInput,
    keys::{Key, Mods},
    types::text_input::{Edit, Preedit, Replacement},
    ui::text_editor::TextEditor,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct Reference {
    qt_version: String,
    table_editor: String,
    scenarios: Vec<Scenario>,
}
#[derive(Deserialize)]
struct Scenario {
    name: String,
    mode: String,
    initial: String,
    operations: Vec<Operation>,
    states: Vec<State>,
}
#[derive(Deserialize)]
struct Operation {
    kind: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    preedit: String,
    #[serde(default)]
    commit: String,
    #[serde(default)]
    cursor: i32,
    #[serde(default)]
    replacement_start_utf16: isize,
    #[serde(default)]
    replacement_length_utf16: usize,
}
#[derive(Debug, Deserialize, PartialEq, Eq)]
struct State {
    text: String,
    cursor: usize,
    anchor: usize,
    selected: String,
    undo: bool,
    redo: bool,
}

/// 【窗口输入法测试】【状态采集】返回 editor 的真实业务内容、字节位置与历史，不读取候选显示文本
fn state(editor: &TextEditor) -> State {
    let (cursor, anchor) = editor.byte_selection();
    State {
        text: editor.text(),
        cursor,
        anchor,
        selected: editor.selected_text().unwrap_or_default(),
        undo: editor.can_undo(),
        redo: editor.can_redo(),
    }
}

/// 【窗口输入法测试】【单位转换】按 text 和 units 将 Qt UTF-16 单元转换为合法字节，非法边界触发测试失败
fn byte_at(text: &str, units: usize) -> usize {
    let mut current = 0;
    for (index, ch) in text.char_indices() {
        if current == units {
            return index;
        }
        current += ch.len_utf16();
    }
    assert_eq!(current, units);
    text.len()
}

/// 【窗口输入法测试】【事件转换】按 editor 当前正文与 operation 返回协议字节事件；不使用预期状态修正实际位置
fn edit(editor: &TextEditor, operation: &Operation) -> Edit {
    let mut text = editor.text();
    let (mut cursor, anchor) = editor.byte_selection();
    let editing = !operation.commit.is_empty()
        || operation.replacement_length_utf16 != 0
        || operation.preedit != editor.preedit().map_or("", |value| &value.text);
    if editing && cursor != anchor {
        let (start, end) = (cursor.min(anchor), cursor.max(anchor));
        text.replace_range(start..end, "");
        cursor = start;
    }
    let units = text[..cursor].encode_utf16().count();
    let start_units = units
        .checked_add_signed(operation.replacement_start_utf16)
        .unwrap();
    let start = byte_at(&text, start_units);
    let end = byte_at(&text, start_units + operation.replacement_length_utf16);
    Edit {
        commit: operation.commit.clone(),
        preedit: Preedit {
            text: operation.preedit.clone(),
            cursor: (operation.cursor >= 0)
                .then_some((operation.cursor as usize, operation.cursor as usize)),
        },
        replacement: (start != cursor || end != start).then_some(Replacement {
            start: start as isize - cursor as isize,
            length: end - start,
        }),
    }
}

/// 【窗口输入法测试】【操作回放】将 operation 投递给 editor，fonts 用于生产排版；不模拟确认后的业务状态
fn apply(editor: &mut TextEditor, fonts: &mut FontSystem, operation: &Operation) {
    match operation.kind.as_str() {
        "input" => {
            let event = edit(editor, operation);
            assert!(editor.input_method(fonts, event).accepted);
        }
        "undo" => {
            editor.undo(fonts);
        }
        "redo" => {
            editor.redo(fonts);
        }
        "paste" => {
            editor.insert(fonts, &operation.text);
        }
        "cut" => {
            editor.delete_selection(fonts);
        }
        "select" => editor.select_all(fonts),
        "type" => {
            for ch in operation.text.chars() {
                editor.key(
                    fonts,
                    &KeyInput {
                        key: Some(Key::Char(ch)),
                        text: ch.to_string(),
                        mods: Mods::default(),
                    },
                );
            }
        }
        "left" | "backspace" | "enter" => {
            let key = match operation.kind.as_str() {
                "left" => Key::Left,
                "backspace" => Key::Backspace,
                _ => Key::Enter,
            };
            editor.key(
                fonts,
                &KeyInput {
                    key: Some(key),
                    text: String::new(),
                    mods: Mods::default(),
                },
            );
        }
        unknown => panic!("unknown Qt operation {unknown}"),
    }
}

/// 【窗口输入法测试】【Qt 真实状态】逐次比较真实输入法投递后的文本、选择、光标与历史；候选不进入业务输出
#[test]
fn qt_text_edit_and_line_edit_input_method_states_match() {
    let reference: Reference =
        serde_json::from_str(include_str!("fixtures/qt-window-input-method.json")).unwrap();
    assert!(!reference.qt_version.is_empty());
    assert_eq!(reference.table_editor, "QLineEdit");
    assert_eq!(reference.scenarios.len(), 35);
    for scenario in reference.scenarios {
        let mut fonts = mark_shot::fonts::bundled();
        let mut editor = if scenario.mode == "singleline" {
            TextEditor::new_single_line(&scenario.initial, &mut fonts)
        } else {
            TextEditor::new(&scenario.initial, &mut fonts)
        };
        assert!(editor.set_byte_selection(
            &mut fonts,
            scenario.states[0].cursor,
            scenario.states[0].anchor
        ));
        assert_eq!(
            state(&editor),
            scenario.states[0],
            "{} {} initial",
            scenario.mode,
            scenario.name
        );
        assert_eq!(scenario.states.len(), scenario.operations.len() + 1);
        for (index, operation) in scenario.operations.iter().enumerate() {
            apply(&mut editor, &mut fonts, operation);
            assert_eq!(
                state(&editor),
                scenario.states[index + 1],
                "{} {} operation {} {}",
                scenario.mode,
                scenario.name,
                index,
                operation.kind
            );
        }
    }
}
