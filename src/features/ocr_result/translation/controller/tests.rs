//! 【OCR 翻译测试】【实际窗口任务】使用私有命令和窗口事件验证原文隔离、独立编辑器及一次结果交付

use super::*;
use crate::{
    backend::window::{KeyInput, WindowContent, WindowEvent, WindowPopupAction, WindowWake},
    features::ocr_result::{
        document::OcrDocument,
        layout::Control,
        translation::input::{self, Field},
    },
    keys::{Key, Mods},
    types::{
        popup::PopupEvent,
        text_input::{Edit, Preedit},
    },
    ui::text_editor::EditEvent,
};

/// 【OCR 翻译测试】【私有窗口】按 command 创建临时配置与未映射窗口，返回两者；所有任务使用本地命令，不读取环境凭据
pub(in crate::features::ocr_result) fn window(command: &str) -> (tempfile::TempDir, ResultWindow) {
    let root = tempfile::tempdir().unwrap();
    let config = crate::config::Translation {
        command: command.into(),
        target_language: "English".into(),
        timeout_ms: 1000,
        ..Default::default()
    };
    let path = root.path().join("config.toml");
    let value = toml::Value::try_from(&config).unwrap();
    let mut document = toml::Table::new();
    document.insert("translation".into(), value);
    std::fs::write(&path, toml::to_string(&document).unwrap()).unwrap();
    let mut window = ResultWindow::new(OcrDocument {
        text: "alpha\nbeta".into(),
        png: crate::utils::encode_png(&tiny_skia::Pixmap::new(2, 2).unwrap()),
        lines: vec![],
    })
    .unwrap();
    window.translation = Translation::new(&mut window.fonts, config);
    window.translation.config_path = Some(path);
    window.connected(WindowWake::detached().unwrap());
    window.refresh_layout();
    (root, window)
}

/// 【OCR 翻译测试】【真实完成等待】在实际期限内向 window 交付 Wake 并等待任务完成，无返回值，超时直接失败
fn finish(window: &mut ResultWindow) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while window.translation.state.busy() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
        window.event(WindowEvent::Wake);
    }
    assert!(!window.translation.state.busy());
}

/// 【OCR 翻译测试】【固定按键】按 key、text、mods 返回窗口按键事件，不交付剪贴板内容
fn key(key: Option<Key>, text: &str, mods: Mods) -> WindowEvent {
    WindowEvent::Key(KeyInput {
        key,
        text: text.into(),
        mods,
    })
}

/// 【OCR 翻译测试】【候选快照隔离】任务使用确认原文，忙碌中原文继续编辑，完成只更新译文并显示过期提示，无参数、无返回值
#[test]
fn real_task_preserves_source_composition_and_later_source_edits() {
    let command = "sleep 0.2; printf '%s' '{\"tokens\":[{\"text\":\"translated\"}]}' < {input}";
    let (_root, mut window) = window(command);
    let source_id = window.source.input_identity();
    let translation_id = window.translation.editor.input_identity();
    let language_id = window.translation.language.input_identity();
    assert!(
        source_id != translation_id && source_id != language_id && translation_id != language_id
    );
    window.source.set_byte_selection(&mut window.fonts, 10, 10);
    window.event(WindowEvent::TextInput(Edit {
        preedit: Preedit {
            text: "candidate".into(),
            cursor: Some((9, 9)),
        },
        commit: String::new(),
        replacement: None,
    }));
    assert_eq!(window.source.text(), "alpha\nbeta");
    assert!(window.source.preedit().is_some());
    window.event(key(
        Some(Key::Enter),
        "",
        Mods {
            ctrl: true,
            ..Default::default()
        },
    ));
    assert!(window.translation.state.busy());
    assert!(window.source.preedit().is_some());
    assert_eq!(window.text_input_target(), Some(source_id));
    window.event(WindowEvent::TextInput(Edit {
        preedit: Preedit::default(),
        commit: "!".into(),
        replacement: None,
    }));
    assert_eq!(window.source.text(), "alpha\nbeta!");
    finish(&mut window);
    assert_eq!(window.translation.editor.text(), "translated");
    assert_eq!(window.source.text(), "alpha\nbeta!");
    assert!(
        window
            .translation
            .state
            .notice()
            .starts_with("Source or language changed")
    );
    assert_ne!(window.translation.editor.input_identity(), translation_id);
    window.focus_control(FocusTarget::Translation);
    assert_eq!(
        window.text_input_target(),
        Some(window.translation.editor.input_identity())
    );
    window.event(key(None, " edited", Mods::default()));
    assert_eq!(window.source.text(), "alpha\nbeta!");
    assert_eq!(window.translation.editor.text(), " editedtranslated");
    window.event(key(
        Some(Key::Char('z')),
        "",
        Mods {
            ctrl: true,
            ..Default::default()
        },
    ));
    assert_eq!(window.translation.editor.text(), "translated");
    assert_eq!(window.source.text(), "alpha\nbeta!");
}

/// 【OCR 翻译测试】【只读范围】忙碌译文允许选择复制但不接受修改或输入法，语言禁用，原文不受冻结，无参数、无返回值
#[test]
fn busy_task_freezes_only_translation_edits_and_language() {
    let (_root, mut window) =
        window("sleep 0.3; printf '%s' '{\"tokens\":[{\"text\":\"new\"}]}' < {input}");
    window.translation.editor.insert(&mut window.fonts, "old");
    window.control(Control::Translate);
    window.focus_control(FocusTarget::Translation);
    window.translation.editor.select_all(&mut window.fonts);
    assert_eq!(window.text_input_target(), None);
    assert_eq!(
        input::key(
            &mut window,
            &KeyInput {
                key: Some(Key::Char('c')),
                text: String::new(),
                mods: Mods {
                    ctrl: true,
                    ..Default::default()
                }
            }
        ),
        EditEvent::Copy("old".into())
    );
    for key in [
        Some(Key::Delete),
        Some(Key::Backspace),
        None,
        Some(Key::Char('v')),
        Some(Key::Char('z')),
    ] {
        window.event(super::tests::key(
            key,
            "attempt",
            if matches!(key, Some(Key::Char(_))) {
                Mods {
                    ctrl: true,
                    ..Default::default()
                }
            } else {
                Mods::default()
            },
        ));
        assert_eq!(window.translation.editor.text(), "old");
    }
    window.focus_control(FocusTarget::Language);
    let target = window.translation.language.text();
    window.event(key(None, "attempt", Mods::default()));
    assert_eq!(window.translation.language.text(), target);
    assert_eq!(window.text_input_target(), None);
    window.focus_control(FocusTarget::Content);
    window.event(key(None, "source edit", Mods::default()));
    assert_eq!(window.translation.editor.text(), "old");
    assert_eq!(window.source.text(), "source editalpha\nbeta");
    window.control(Control::Translate);
    assert!(!window.translation.state.busy());
    assert_eq!(window.translation.editor.text(), "old");
    assert_eq!(window.translation.state.notice(), "Translation canceled");
}

/// 【OCR 翻译测试】【显示与页面保留】已有译文收起、重开和切页均保留编辑历史，翻译焦点恢复到自己的编辑器，不再次创建任务，无参数、无返回值
#[test]
fn reopening_and_view_switch_keep_translation_history_without_requesting_again() {
    let (_root, mut window) = window("exit 8");
    window
        .translation
        .editor
        .insert(&mut window.fonts, "manual translation");
    window.control(Control::TranslationToggle);
    assert!(window.translation.task.is_none());
    window.refresh_layout();
    window.focus_control(FocusTarget::Translation);
    window.control(Control::View(View::Image));
    window.control(Control::View(View::Text));
    assert!(window.focus_target == FocusTarget::Translation);
    window.control(Control::TranslationToggle);
    window.control(Control::TranslationToggle);
    assert!(window.translation.task.is_none());
    assert_eq!(window.translation.editor.text(), "manual translation");
    assert!(window.translation.editor.can_undo());
    window.control(Control::TranslationUndo);
    assert_eq!(window.translation.editor.text(), "");
    assert_eq!(window.source.text(), "alpha\nbeta");
}

/// 【OCR 翻译测试】【原生选项列表】八行视区仍能通过 End 选择第九项，弹层帧有独立身份并保持分数缩放有效，确认只保存目标语言，无参数、无返回值
#[test]
fn native_language_popup_selects_ninth_option_and_preserves_document() {
    let (root, mut window) = window("exit 8");
    window.translation.editor.insert(&mut window.fonts, "keep");
    window.control(Control::TranslationToggle);
    window.refresh_layout();
    window.control(Control::LanguageMenu);
    let id = window.popup_id().unwrap();
    for scale in [1.0, 1.2, 2.0] {
        crate::types::popup::validate(&window.popup_frame(scale).unwrap()).unwrap();
    }
    let chord = crate::keys::parse("End").unwrap();
    assert_eq!(
        window.popup_event(
            id,
            PopupEvent::Key {
                chord: Some(chord),
                text: None
            }
        ),
        WindowPopupAction::Redraw
    );
    assert_eq!(
        window.popup_event(
            id,
            PopupEvent::Key {
                chord: Some(crate::keys::parse("Return").unwrap()),
                text: None
            }
        ),
        WindowPopupAction::Activate(8)
    );
    window.popup_closed(id, Some(8));
    assert_eq!(window.translation.language.text(), "Russian");
    assert_eq!(
        crate::config::translation::read_at(&root.path().join("config.toml"))
            .unwrap()
            .target_language,
        "Russian"
    );
    assert_eq!(window.source.text(), "alpha\nbeta");
    assert_eq!(window.translation.editor.text(), "keep");
    assert!(window.translation.task.is_none());
}

/// 【OCR 翻译测试】【任务后菜单隔离】程序替换译文身份后旧菜单不得删除新结果，关闭动作再次核对所属编辑器，无参数、无返回值
#[test]
fn stale_translation_popup_cannot_edit_the_replacement_editor() {
    let (_root, mut window) = window("exit 8");
    window.translation.editor.insert(&mut window.fonts, "old");
    window.translation.editor.select_all(&mut window.fonts);
    let rows = crate::features::ocr_result::source_menu_actions::rows_for(
        &window,
        Field::Translation,
        false,
    );
    crate::features::ocr_result::source_menu::open_rows(
        &mut window,
        (200.0, 100.0),
        crate::features::ocr_result::source_menu::Kind::Translation,
        rows,
    );
    window.popup_editor_id = Some((
        Field::Translation,
        window.translation.editor.input_identity(),
    ));
    let id = window.popup_id().unwrap();
    window.translation.editor =
        TextEditor::new_with_style("replacement", &mut window.fonts, style(false));
    window.translation.editor.select_all(&mut window.fonts);
    window.popup_closed(id, Some(6));
    assert_eq!(window.translation.editor.text(), "replacement");
    assert_eq!(window.source.text(), "alpha\nbeta");
}

/// 【OCR 翻译测试】【候选不改变业务状态】取消显示候选和没有历史的撤销不触发原文变化，原版取消提示应保留，无参数、无返回值
#[test]
fn canceled_notice_survives_composition_cancellation_and_noop_undo() {
    let (_root, mut window) =
        window("sleep 0.1; printf '%s' '{\"tokens\":[{\"text\":\"translated\"}]}' < {input}");
    window.control(Control::Translate);
    finish(&mut window);
    window.focus_control(FocusTarget::Content);
    window.event(WindowEvent::TextInput(Edit {
        preedit: Preedit {
            text: "candidate".into(),
            cursor: Some((9, 9)),
        },
        commit: String::new(),
        replacement: None,
    }));
    window.control(Control::Translate);
    window.control(Control::Translate);
    assert_eq!(window.translation.state.notice(), "Translation canceled");
    assert!(!window.source.can_undo());
    window.event(key(
        Some(Key::Char('z')),
        "",
        Mods {
            ctrl: true,
            ..Default::default()
        },
    ));
    assert_eq!(window.source.text(), "alpha\nbeta");
    assert_eq!(window.translation.state.notice(), "Translation canceled");
}
