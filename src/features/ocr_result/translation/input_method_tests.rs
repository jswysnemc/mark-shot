//! 【OCR 翻译输入测试】【原版按键】真实 Qt 多行编辑器只使用 Ctrl+Shift+Z 重做，原文与译文保持相同分派

use super::*;
use crate::{
    backend::window::{KeyInput, LEFT_BUTTON, WindowContent, WindowEvent},
    features::ocr_result::control_input::FocusTarget,
    keys::{Key, Mods},
    types::text_input::{Batch, Edit, Focus, Preedit, Replacement, Update},
    ui::text_editor::TextEditor,
};
use serde_json::{Value, json};

/// 【OCR 翻译输入测试】【固定按键】按 character、shift 返回 Ctrl 组合事件，不生成普通插入文字
fn key(character: char, shift: bool) -> WindowEvent {
    WindowEvent::Key(KeyInput {
        key: Some(Key::Char(character)),
        text: String::new(),
        mods: Mods {
            ctrl: true,
            shift,
            ..Default::default()
        },
    })
}

/// 【OCR 翻译输入测试】【重做别名】原文和译文均拒绝 Ctrl+Y，但 Ctrl+Shift+Z 恢复一次输入法确认，无参数、无返回值
#[test]
fn qt_multiline_fields_ignore_ctrl_y_and_redo_with_ctrl_shift_z() {
    for field in [input::Field::Source, input::Field::Translation] {
        let (_root, mut window) = controller::tests::window("exit 8");
        window.source = TextEditor::new_with_style("seed", &mut window.fonts, style(false));
        window.translation.editor =
            TextEditor::new_with_style("seed", &mut window.fonts, style(false));
        window.translation.state.visible = true;
        window.refresh_layout();
        window.focus_control(if field == input::Field::Source {
            FocusTarget::Content
        } else {
            FocusTarget::Translation
        });
        input::with_editor(&mut window, field, |editor, fonts| {
            editor.set_byte_selection(fonts, 4, 4)
        });
        window.event(WindowEvent::TextInput(Edit {
            commit: "X".into(),
            ..Default::default()
        }));
        assert_eq!(input::editor(&window, field).unwrap().text(), "seedX");
        window.event(key('z', false));
        assert_eq!(input::editor(&window, field).unwrap().text(), "seed");
        window.event(key('y', false));
        assert_eq!(
            input::editor(&window, field).unwrap().text(),
            "seed",
            "Ctrl+Y changed a Qt multiline field"
        );
        window.event(key('z', true));
        assert_eq!(input::editor(&window, field).unwrap().text(), "seedX");
    }
}

/// 【OCR 翻译输入测试】【确认事件】按 preedit、commit 和 replacement 返回完整输入法事件，位置使用生产接口的 UTF-8 字节
fn edit(preedit: &str, commit: &str, replacement: Option<Replacement>) -> WindowEvent {
    WindowEvent::TextInput(Edit {
        preedit: Preedit {
            text: preedit.into(),
            cursor: Some((preedit.len(), preedit.len())),
        },
        commit: commit.into(),
        replacement,
    })
}

/// 【OCR 翻译输入测试】【控件记录】按 window、field 返回确认文字、UTF-16 光标、历史和输入资格，Qt 多行候选另行读取，不包含绘制位置
fn record(
    window: &crate::features::ocr_result::window::ResultWindow,
    field: input::Field,
) -> Value {
    let editor = input::editor(window, field).unwrap();
    let text = editor.text();
    let (cursor, anchor) = editor.byte_selection();
    let mut value = json!({"text":text,"cursor":text[..cursor].encode_utf16().count(),"anchor":text[..anchor].encode_utf16().count(),
        "undo":editor.can_undo(),"redo":editor.can_redo(),"input_enabled":input::editable(window,field)});
    if field != input::Field::Language {
        value["preedit"] = json!(editor.preedit().map_or("", |value| value.text.as_str()));
    }
    value
}

/// 【OCR 翻译输入测试】【完整编辑序列】回放两个宽度下的全部确认、历史、只读与语言保存状态，逐项对照真实 Qt 控件查询，无参数、无返回值
#[test]
fn editor_input_sequence_matches_actual_qt_translation_records() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/qt-ocr-translation-input.json"
    ))
    .unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let (_root, mut window) = controller::tests::window("sleep 0.6; exit 8");
        window.layout.size = (case["width"].as_f64().unwrap() as f32, 520.0);
        window.source = TextEditor::new_with_style("base", &mut window.fonts, style(false));
        window.translation.editor =
            TextEditor::new_with_style("draft", &mut window.fonts, style(false));
        window
            .translation
            .editor
            .set_byte_selection(&mut window.fonts, 5, 5);
        window.translation.state.visible = true;
        window.focus_control(FocusTarget::Translation);
        window.refresh_layout();
        for step in case["states"].as_array().unwrap() {
            let name = step["name"].as_str().unwrap();
            match name {
                "initial" => {}
                "translation-preedit" => {
                    window.event(edit("ni", "", None));
                }
                "translation-commit-x" => {
                    window.event(edit("hao", "X", None));
                }
                "translation-commit-y" => {
                    window.event(edit("", "Y", None));
                }
                "translation-undo-y" | "translation-undo-x" | "source-undo" => {
                    window.event(key('z', false));
                }
                "translation-ctrl-y-no-redo" | "source-ctrl-y-no-redo" => {
                    window.event(key('y', false));
                }
                "translation-redo-x" | "translation-redo-y" | "source-redo" => {
                    window.event(key('z', true));
                }
                "translation-supplementary" => {
                    window.event(edit("", "𠮷", None));
                }
                "translation-replacement" => {
                    window.event(edit(
                        "",
                        "你",
                        Some(Replacement {
                            start: -4,
                            length: 4,
                        }),
                    ));
                }
                "source-preedit" => {
                    window.focus_control(FocusTarget::Content);
                    window.source.set_byte_selection(&mut window.fonts, 4, 4);
                    window.event(edit("source-candidate", "", None));
                }
                "busy-source-preedit" | "busy-from-language" => {
                    window.event(WindowEvent::Key(KeyInput {
                        key: Some(Key::Enter),
                        text: String::new(),
                        mods: Mods {
                            ctrl: true,
                            ..Default::default()
                        },
                    }));
                }
                "busy-source-commit" => {
                    window.event(edit("", "!", None));
                }
                "busy-translation-rejected" => {
                    window.focus_control(FocusTarget::Translation);
                    window.event(WindowEvent::Key(KeyInput {
                        key: None,
                        text: "forbidden".into(),
                        mods: Mods::default(),
                    }));
                }
                "canceled" | "language-task-canceled" => {
                    window.control(super::super::layout::Control::Translate);
                }
                "language-preedit-selection" => {
                    window.focus_control(FocusTarget::Language);
                    window.translation.language.select_all(&mut window.fonts);
                    window.event(edit("free", "", None));
                }
                "language-commit" => {
                    window.event(edit("", "Esperanto", None));
                }
                "language-return-saved" => {
                    window.event(WindowEvent::Key(KeyInput {
                        key: Some(Key::Enter),
                        text: String::new(),
                        mods: Mods::default(),
                    }));
                }
                "language-preedit-before-start" => {
                    window.event(edit("target-candidate", "", None));
                }
                "language-restored" => {
                    window.focus_control(FocusTarget::Language);
                    window.event(edit("", "", None));
                }
                _ => panic!("unhandled Qt editor step {name}"),
            }
            let expected = &step["state"];
            for (label, field) in [
                ("source", input::Field::Source),
                ("translation", input::Field::Translation),
                ("language", input::Field::Language),
            ] {
                assert_eq!(
                    record(&window, field),
                    expected[label],
                    "{} {name} {label}",
                    case["width"]
                );
            }
            assert_eq!(
                window.translation.state.busy(),
                expected["busy"].as_bool().unwrap(),
                "{name}"
            );
            assert_eq!(
                window.translation.saved_target,
                expected["saved_target"].as_str().unwrap(),
                "{name}"
            );
        }
    }
}

/// 【OCR 翻译输入测试】【页面候选】实际页签按下保留原文和译文候选显示，隐藏与恢复期间旧协议目标不能修改正文，无参数、无返回值
#[test]
fn hidden_text_views_keep_preedit_without_accepting_old_protocol_batches() {
    for field in [input::Field::Source, input::Field::Translation] {
        let (_root, mut window) = controller::tests::window("exit 8");
        window.source = TextEditor::new_with_style("seed", &mut window.fonts, style(false));
        window.translation.editor =
            TextEditor::new_with_style("seed", &mut window.fonts, style(false));
        window.translation.state.visible = true;
        window.refresh_layout();
        window.focus_control(if field == input::Field::Source {
            FocusTarget::Content
        } else {
            FocusTarget::Translation
        });
        input::with_editor(&mut window, field, |editor, fonts| {
            editor.set_byte_selection(fonts, 4, 4)
        });
        window.event(edit("view-candidate", "", None));
        let mut target = crate::features::text::input_method::focus::Target::default();
        target.enter(Focus {
            id: 100,
            monitor_idx: 0,
        });
        target.reconcile(window.text_input_target());
        let previous = target.id.unwrap();
        let update = Update {
            target_id: previous,
            serial: 0,
            sync_allowed: true,
            batch: Batch {
                commit: "stale".into(),
                ..Default::default()
            },
        };
        for view in [
            super::super::window::View::Image,
            super::super::window::View::Text,
        ] {
            let tab = window
                .layout
                .titlebar
                .tabs
                .iter()
                .find(|(candidate, _)| *candidate == view)
                .unwrap()
                .1;
            window.event(WindowEvent::PointerButton {
                position: (f64::from(tab.x() + 10.0), f64::from(tab.y() + 10.0)),
                button: LEFT_BUTTON,
                pressed: true,
            });
            if target.reconcile(window.text_input_target()) {
                window.suspend_text_input();
            }
            assert_eq!(
                input::editor(&window, field)
                    .unwrap()
                    .preedit()
                    .unwrap()
                    .text,
                "view-candidate"
            );
            assert!(!target.accept(&update));
            if view == super::super::window::View::Image {
                assert!(window.text_input_target().is_none());
                window.event(edit("ignored", "hidden-stale", None));
                assert_eq!(input::editor(&window, field).unwrap().text(), "seed");
            }
        }
        assert_ne!(target.id, Some(previous));
        window.event(edit("", "X", None));
        assert_eq!(input::editor(&window, field).unwrap().text(), "seedX");
        assert!(input::editor(&window, field).unwrap().preedit().is_none());
    }
}

/// 【OCR 翻译输入测试】【菜单候选】原文和译文菜单保留候选外观但暂停输入，关闭后新身份拒绝旧提交，正常确认仍可撤销，无参数、无返回值
#[test]
fn multiline_menus_keep_candidate_display_and_reject_stale_protocol_commits() {
    for (field, focus, kind) in [
        (
            input::Field::Source,
            FocusTarget::Content,
            super::super::source_menu::Kind::Text,
        ),
        (
            input::Field::Translation,
            FocusTarget::Translation,
            super::super::source_menu::Kind::Translation,
        ),
    ] {
        let (_root, mut window) = controller::tests::window("exit 8");
        window.source = TextEditor::new_with_style("seed", &mut window.fonts, style(false));
        window.translation.editor =
            TextEditor::new_with_style("seed", &mut window.fonts, style(false));
        window.translation.state.visible = true;
        window.refresh_layout();
        window.focus_control(focus);
        input::with_editor(&mut window, field, |editor, fonts| {
            editor.set_byte_selection(fonts, 4, 4)
        });
        window.event(edit("menu-candidate", "", None));
        let mut target = crate::features::text::input_method::focus::Target::default();
        target.enter(Focus {
            id: 100,
            monitor_idx: 0,
        });
        target.reconcile(window.text_input_target());
        let previous = target.id.unwrap();
        let rows = super::super::source_menu_actions::rows_for(&window, field, false);
        super::super::source_menu::open_rows(&mut window, (120.0, 120.0), kind, rows);
        let id = window.source_menu.as_ref().unwrap().id;
        if target.reconcile(window.text_input_target()) {
            window.suspend_text_input();
        }
        assert!(window.text_input_target().is_none());
        assert_eq!(
            input::editor(&window, field)
                .unwrap()
                .preedit()
                .map(|value| value.text.as_str()),
            Some("menu-candidate")
        );
        window.event(edit("ignored", "stale-menu", None));
        assert_eq!(input::editor(&window, field).unwrap().text(), "seed");
        super::super::source_menu::closed(&mut window, id, None);
        if target.reconcile(window.text_input_target()) {
            window.suspend_text_input();
        }
        assert_ne!(target.id, Some(previous));
        assert!(!target.accept(&Update {
            target_id: previous,
            serial: 0,
            sync_allowed: true,
            batch: Batch {
                commit: "stale-menu".into(),
                ..Default::default()
            },
        }));
        assert_eq!(
            input::editor(&window, field)
                .unwrap()
                .preedit()
                .unwrap()
                .text,
            "menu-candidate"
        );
        window.event(edit("", "X", None));
        assert_eq!(input::editor(&window, field).unwrap().text(), "seedX");
        window.event(key('z', false));
        assert_eq!(input::editor(&window, field).unwrap().text(), "seed");
    }
}

/// 【OCR 翻译输入测试】【真实失焦】临时停用保留的多行候选在真实失焦及字段切换时仍清理，避免候选泄漏到其他控件，无参数、无返回值
#[test]
fn suspended_multiline_candidates_are_canceled_on_window_blur_and_field_change() {
    for focus in [FocusTarget::Content, FocusTarget::Translation] {
        let (_root, mut window) = controller::tests::window("exit 8");
        window.translation.state.visible = true;
        window.refresh_layout();
        window.focus_control(focus);
        let field = input::field(&window).unwrap();
        window.event(edit("candidate", "", None));
        window.suspend_text_input();
        assert!(input::editor(&window, field).unwrap().preedit().is_some());
        window.event(WindowEvent::Focused(false));
        assert!(input::editor(&window, field).unwrap().preedit().is_none());
        assert!(window.text_input_target().is_none());
        window.event(WindowEvent::Focused(true));
        window.event(edit("candidate", "", None));
        window.focus_control(FocusTarget::Language);
        assert!(input::editor(&window, field).unwrap().preedit().is_none());
        window.event(edit("language-candidate", "", None));
        assert!(window.translation.language.preedit().is_some());
        assert!(window.suspend_text_input());
        assert!(window.translation.language.preedit().is_none());
    }
}

/// 【OCR 翻译输入测试】【修饰键独立事件】实际 Ctrl、Shift、Alt、Logo 按下没有编辑按键时不打断各字段候选，原文快捷翻译仍保留候选，无参数、无返回值
#[test]
fn modifier_only_key_presses_keep_preedit_before_the_translation_shortcut() {
    for focus in [
        FocusTarget::Content,
        FocusTarget::Translation,
        FocusTarget::Language,
    ] {
        let (_root, mut window) = controller::tests::window("sleep 0.6; exit 8");
        window.translation.state.visible = true;
        window.refresh_layout();
        window.focus_control(focus);
        let field = input::field(&window).unwrap();
        window.event(edit("candidate", "", None));
        for mods in [
            Mods::default(),
            Mods {
                ctrl: true,
                ..Default::default()
            },
            Mods {
                shift: true,
                ..Default::default()
            },
            Mods {
                alt: true,
                ..Default::default()
            },
            Mods {
                logo: true,
                ..Default::default()
            },
        ] {
            window.event(WindowEvent::Key(KeyInput {
                key: None,
                text: String::new(),
                mods,
            }));
            assert_eq!(
                input::editor(&window, field)
                    .unwrap()
                    .preedit()
                    .map(|value| value.text.as_str()),
                Some("candidate")
            );
        }
        if focus == FocusTarget::Content {
            window.event(WindowEvent::Key(KeyInput {
                key: Some(Key::Enter),
                text: String::new(),
                mods: Mods {
                    ctrl: true,
                    ..Default::default()
                },
            }));
            assert!(window.translation.state.busy());
            assert_eq!(window.source.preedit().unwrap().text, "candidate");
        }
    }
}
