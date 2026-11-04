use super::*;
use crate::{
    backend::wayland::window::{KeyInput, LEFT_BUTTON, WindowContent, WindowEvent},
    features::ocr_result::{document::OcrDocument, table::Table, table_view::TableView},
    keys::{Key, Mods},
    types::text_input::{Batch, Preedit, Replacement, Update},
};
use tiny_skia::Rect;

/// 【结果输入法测试】【私有状态】创建无模型、无存储副作用的未连接结果窗口，返回可测试状态
fn window() -> ResultWindow {
    ResultWindow::new(OcrDocument {
        text: "base".into(),
        png: crate::utils::encode_png(&tiny_skia::Pixmap::new(2, 2).unwrap()),
        lines: vec![],
    })
    .unwrap()
}

/// 【结果输入法测试】【候选事件】构造 preedit 和 commit 对应的窗口输入，不连接合成器，返回事件
fn input(preedit: &str, commit: &str) -> WindowEvent {
    WindowEvent::TextInput(Edit {
        preedit: Preedit {
            text: preedit.into(),
            cursor: Some((preedit.len(), preedit.len())),
        },
        commit: commit.into(),
        replacement: None,
    })
}

/// 【结果输入法测试】【原文隔离】候选改变显示位置但不改变业务文本，连续确认分别撤销
#[test]
fn source_preedit_and_undo_use_qt_text_edit_semantics() {
    let mut state = window();
    state.source.set_byte_selection(&mut state.fonts, 4, 4);
    let focus = Focus {
        id: 100,
        monitor_idx: 0,
    };
    let before = snapshot(&mut state, focus, 200, None, ChangeCause::Other).unwrap();
    assert_eq!(state.event(input("ni", "")), WindowAction::Redraw);
    assert_eq!(state.source.text(), "base");
    let after = snapshot(
        &mut state,
        focus,
        200,
        Some(&before),
        ChangeCause::InputMethod,
    )
    .unwrap();
    assert_eq!(after.surrounding, before.surrounding);
    assert!(after.cursor_rect.unwrap().0 > before.cursor_rect.unwrap().0);
    assert!(snapshot(&mut state, focus, 200, Some(&after), ChangeCause::Other).is_none());
    state.event(input("hao", "X"));
    state.event(input("", "Y"));
    assert_eq!(state.source.text(), "baseXY");
    state.source.undo(&mut state.fonts);
    assert_eq!(state.source.text(), "baseX");
    state.source.undo(&mut state.fonts);
    assert_eq!(state.source.text(), "base");
}

/// 【结果输入法测试】【输入资格】原图、按钮焦点、格式菜单和失焦窗口不能接收输入法事件
#[test]
fn non_editor_owners_never_accept_input_method_updates() {
    let mut state = window();
    state.event(input("ni", ""));
    state.event(WindowEvent::PointerButton {
        position: (98.0, 56.0),
        button: LEFT_BUTTON,
        pressed: true,
    });
    assert_eq!(state.view, View::Image);
    assert!(identity(&state).is_none());
    assert!(state.source.preedit().is_none());
    state.event(input("ignored", "wrong"));
    assert_eq!(state.source.text(), "base");
    state.view = View::Text;
    state.focus_target =
        super::super::control_input::FocusTarget::Button(super::super::layout::Control::Copy);
    assert!(identity(&state).is_none());
    assert_eq!(
        state.event(WindowEvent::Key(KeyInput {
            key: Some(Key::Char('x')),
            text: "x".into(),
            mods: Mods::default()
        })),
        WindowAction::None
    );
    state.focus_target = super::super::control_input::FocusTarget::Content;
    state.menu = true;
    assert!(identity(&state).is_none());
    state.menu = false;
    state.event(input("ni", ""));
    state.event(WindowEvent::Focused(false));
    assert!(state.source.preedit().is_none());
    assert!(identity(&state).is_none());
}

/// 【结果输入法测试】【目标隔离】切页后重新回到同一编辑器也分配新协议目标，旧批次不能修改新目标
#[test]
fn returning_to_the_source_uses_a_fresh_protocol_target() {
    let mut state = window();
    let mut target = crate::features::text::input_method::focus::Target::default();
    target.enter(Focus {
        id: 100,
        monitor_idx: 0,
    });
    target.reconcile(identity(&state));
    let old = target.id.unwrap();
    state.view = View::Image;
    target.reconcile(identity(&state));
    state.view = View::Text;
    target.reconcile(identity(&state));
    assert_ne!(target.id, Some(old));
    assert!(!target.accept(&Update {
        target_id: old,
        serial: 0,
        sync_allowed: true,
        batch: Batch::default()
    }));
    assert_eq!(state.source.text(), "base");
}

/// 【结果输入法测试】【单元格事务】普通选择不立即编辑，确认文字仍是草稿；Enter 才提交，Escape 丢弃
#[test]
fn table_drafts_do_not_change_persistent_cells_before_commit() {
    let mut fonts = crate::fonts::bundled();
    let mut table = TableView::new(Table {
        rows: vec![vec!["cell".into(), "other".into()]],
    });
    table.active = Some((0, 0));
    assert!(table.editor.is_none());
    table.begin_edit(&mut fonts);
    let editor = table.editor.as_mut().unwrap();
    editor.input_method(
        &mut fonts,
        Edit {
            preedit: Preedit {
                text: "ni".into(),
                cursor: Some((2, 2)),
            },
            ..Edit::default()
        },
    );
    assert_eq!(table.table.rows[0][0], "cell");
    assert_eq!(table.editor.as_ref().unwrap().text(), "");
    table.editor.as_mut().unwrap().input_method(
        &mut fonts,
        Edit {
            commit: "X".into(),
            ..Edit::default()
        },
    );
    table.editor.as_mut().unwrap().input_method(
        &mut fonts,
        Edit {
            commit: "Y".into(),
            ..Edit::default()
        },
    );
    assert_eq!(table.table.rows[0][0], "cell");
    table.editor.as_mut().unwrap().undo(&mut fonts);
    assert_eq!(table.editor.as_ref().unwrap().text(), "cell");
    table.editor.as_mut().unwrap().redo(&mut fonts);
    table.finish(true);
    assert_eq!(table.table.rows[0][0], "XY");
    table.begin_edit(&mut fonts);
    table
        .editor
        .as_mut()
        .unwrap()
        .insert(&mut fonts, "discarded");
    table.finish(false);
    assert_eq!(table.table.rows[0][0], "XY");
}

/// 【结果输入法测试】【表格参考】对照真实 QTableWidget 的 Enter、Shift Enter、Tab、Shift Tab 和 Escape 提交行为
#[test]
fn table_navigation_and_commit_match_real_qt_editor_states() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/qt-window-input-method.json"
    ))
    .unwrap();
    let body = Rect::from_xywh(8.0, 78.0, 744.0, 354.0).unwrap();
    for case in reference["table_scenarios"].as_array().unwrap() {
        let kind = case["kind"].as_str().unwrap();
        if kind == "input-method" {
            continue;
        }
        let mut fonts = crate::fonts::bundled();
        let mut table = TableView::new(Table {
            rows: vec![
                vec!["cell".into(), "other".into()],
                vec!["other".into(), "other".into()],
            ],
        });
        table.active = Some((0, 0));
        if kind != "enter-selection" {
            table.key(
                &mut fonts,
                body,
                &KeyInput {
                    key: Some(Key::F(2)),
                    text: String::new(),
                    mods: Mods::default(),
                },
            );
            table.key(
                &mut fonts,
                body,
                &KeyInput {
                    key: Some(Key::Char('X')),
                    text: "X".into(),
                    mods: Mods::default(),
                },
            );
        }
        let key = match kind {
            "tab" | "shift-tab" => Key::Tab,
            "escape" => Key::Escape,
            _ => Key::Enter,
        };
        table.key(
            &mut fonts,
            body,
            &KeyInput {
                key: Some(key),
                text: String::new(),
                mods: Mods {
                    shift: kind.starts_with("shift-"),
                    ..Mods::default()
                },
            },
        );
        let expected = case["states"].as_array().unwrap().last().unwrap();
        assert_eq!(
            table.table.rows[0][0],
            expected["cell"].as_str().unwrap(),
            "{kind}"
        );
        assert_eq!(
            table.editor.is_some(),
            expected["editing"].as_bool().unwrap(),
            "{kind}"
        );
        assert_eq!(
            table.active,
            Some((
                expected["row"].as_u64().unwrap() as usize,
                expected["column"].as_u64().unwrap() as usize
            )),
            "{kind}"
        );
        if kind == "escape" {
            assert_eq!(case["root_escape"].as_bool(), Some(false));
        }
    }
}

/// 【结果输入法测试】【字节拒绝】替换切开扩展汉字时整批拒绝，旧候选与正文保持不变
#[test]
fn malformed_unicode_deletion_keeps_the_previous_preedit() {
    let mut fonts = crate::fonts::bundled();
    let mut editor = crate::ui::text_editor::TextEditor::new("a𠮷", &mut fonts);
    editor.set_byte_selection(&mut fonts, 5, 5);
    editor.input_method(
        &mut fonts,
        Edit {
            preedit: Preedit {
                text: "ni".into(),
                cursor: Some((2, 2)),
            },
            ..Edit::default()
        },
    );
    let result = editor.input_method(
        &mut fonts,
        Edit {
            commit: "X".into(),
            replacement: Some(Replacement {
                start: -1,
                length: 1,
            }),
            ..Edit::default()
        },
    );
    assert!(!result.accepted);
    assert_eq!(editor.text(), "a𠮷");
    assert_eq!(editor.preedit().unwrap().text, "ni");
    assert!(!editor.can_undo());
}
