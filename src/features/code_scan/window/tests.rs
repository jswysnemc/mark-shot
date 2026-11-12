//! 【扫码测试】【只读结果】选择和导航不阻止查看，正文不会被编辑键或组合输入改写

use super::*;
use crate::{
    backend::window::{KeyInput, WindowContent, WindowEvent},
    keys::{Key, Mods},
};

/// 【扫码测试】【编辑隔离】无参数，验证全选后删除、剪切、粘贴和 AltGr 输入都保持识别原文
#[test]
fn result_text_remains_read_only_with_editing_shortcuts() {
    let mut state = ResultWindow::new(Vec::new(), false, false);
    let source = "https://example.com/中文";
    state.editor = TextEditor::new(source, &mut state.fonts);
    state.event(WindowEvent::Key(KeyInput {
        key: Some(Key::Char('a')),
        text: String::new(),
        mods: Mods {
            ctrl: true,
            ..Default::default()
        },
    }));
    assert_eq!(state.editor.selected_text().as_deref(), Some(source));
    for (key, mods, text) in [
        (Key::Delete, Mods::default(), ""),
        (Key::Backspace, Mods::default(), ""),
        (
            Key::Char('x'),
            Mods {
                ctrl: true,
                ..Default::default()
            },
            "",
        ),
        (
            Key::Char('v'),
            Mods {
                ctrl: true,
                ..Default::default()
            },
            "",
        ),
        (
            Key::Char('a'),
            Mods {
                ctrl: true,
                alt: true,
                ..Default::default()
            },
            "ą",
        ),
        (Key::Char('z'), Mods::default(), "z"),
    ] {
        state.event(WindowEvent::Key(KeyInput {
            key: Some(key),
            text: text.into(),
            mods,
        }));
        assert_eq!(state.editor.text(), source);
    }
}
