use cosmic_text::{Action, FontSystem};
use mark_shot::{
    backend::window::KeyInput,
    keys::{Key, Mods},
    ui::text_editor::{EditEvent, TextEditor},
};

/// 【编辑测试】【字体环境】返回只使用项目字体的编辑器及字体系统
fn editor(text: &str) -> (TextEditor, FontSystem) {
    let mut fonts = mark_shot::fonts::bundled();
    (TextEditor::new(text, &mut fonts), fonts)
}

/// 【编辑测试】【Unicode 历史】验证多行中文替换与撤销、重做不破坏字符边界
#[test]
fn multiline_unicode_editing_is_reversible() {
    let (mut editor, mut fonts) = editor("识别原文\nsecond line");
    editor.select_all(&mut fonts);
    assert!(editor.insert(&mut fonts, "替换内容\r\n第三行"));
    assert_eq!(editor.text(), "替换内容\n第三行");
    assert!(editor.undo(&mut fonts));
    assert_eq!(editor.text(), "识别原文\nsecond line");
    assert!(editor.redo(&mut fonts));
    assert_eq!(editor.text(), "替换内容\n第三行");
}

/// 【编辑测试】【粘贴事务】验证一次多行插入只需要一次撤销
#[test]
fn pasted_text_has_one_undo_entry() {
    let (mut editor, mut fonts) = editor("original");
    assert!(editor.insert(&mut fonts, "a\nb\nc\n"));
    assert!(editor.undo(&mut fonts));
    assert_eq!(editor.text(), "original");
    assert!(!editor.can_undo());
}

/// 【编辑测试】【复制边界】验证结果窗口 Ctrl+C 只复制选择，Ctrl+Shift+C 复制全文
#[test]
fn copy_shortcuts_keep_selection_semantics() {
    let (mut editor, mut fonts) = editor("copy this");
    let mut input = KeyInput {
        key: Some(Key::Char('c')),
        text: String::new(),
        mods: Mods {
            ctrl: true,
            ..Mods::default()
        },
    };
    assert_eq!(editor.key(&mut fonts, &input), EditEvent::None);
    input.mods.shift = true;
    assert_eq!(
        editor.key(&mut fonts, &input),
        EditEvent::Copy("copy this".into())
    );
    input.mods.shift = false;
    editor.select_all(&mut fonts);
    assert_eq!(
        editor.key(&mut fonts, &input),
        EditEvent::Copy("copy this".into())
    );
}

/// 【编辑测试】【历史分支】验证撤销后插入新文字会清除重做分支
#[test]
fn a_new_edit_discards_redo() {
    let (mut editor, mut fonts) = editor("");
    editor.insert(&mut fonts, "a");
    editor.undo(&mut fonts);
    editor.insert(&mut fonts, "b");
    assert!(!editor.redo(&mut fonts));
    assert_eq!(editor.text(), "b");
}

/// 【编辑测试】【字符删除】验证组合字符按字素删除，而非切断 UTF-8
#[test]
fn backspace_keeps_grapheme_boundaries() {
    let (mut editor, mut fonts) = editor("");
    editor.insert(&mut fonts, "e\u{301}");
    editor.action(&mut fonts, Action::Backspace);
    assert_eq!(editor.text(), "");
    editor.undo(&mut fonts);
    assert_eq!(editor.text(), "e\u{301}");
}

/// 【编辑测试】【尺寸切换】验证重新排版不会清除内容或历史
#[test]
fn resizing_keeps_text_and_history() {
    let (mut editor, mut fonts) = editor("word");
    editor.insert(&mut fonts, "changed ");
    editor.resize(&mut fonts, 180.0, 90.0);
    editor.resize(&mut fonts, 720.0, 360.0);
    assert!(editor.undo(&mut fonts));
    assert_eq!(editor.text(), "word");
}
