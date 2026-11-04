use super::*;
use crate::{
    backend::wayland::window::{KeyInput, WindowContent, WindowEvent},
    features::ocr_result::{document::OcrDocument, window::View},
    keys::{Chord, Key, Mods},
    types::{
        MouseButton,
        text_input::{Edit, Preedit},
    },
};

/// 【原文菜单测试】【固定窗口】返回正文与原版样本一致的未连接窗口，不启动模型或写入配置，无参数
fn window() -> ResultWindow {
    ResultWindow::new(OcrDocument {
        text: "alpha beta\ngamma".into(),
        png: crate::utils::encode_png(&tiny_skia::Pixmap::new(2, 2).unwrap()),
        lines: vec![],
    })
    .unwrap()
}

/// 【原文菜单测试】【私有菜单】按 state 当前编辑状态构造菜单，clipboard 指定可用资格，返回固定会话编号；不访问系统剪贴板
fn menu(state: &mut ResultWindow, clipboard: bool) -> u64 {
    super::super::input_method::cancel(state);
    let rows = actions::rows(state, clipboard);
    let mut menu = ContextMenu::new(rows, &mut state.fonts, (120.0, 120.0), (760.0, 460.0), 0);
    menu.id = 42;
    state.source_menu = Some(menu);
    42
}

/// 【原文菜单测试】【冻结几何】实际 Qt 原文菜单的顺序、快捷键、行矩形和大小与共享组件一致，无参数、无返回值
#[test]
fn menu_layout_and_shortcuts_match_actual_ocr_pane() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/qt-ocr-source-menu.json"
    ))
    .unwrap();
    let mut state = window();
    state.source.set_byte_selection(&mut state.fonts, 0, 5);
    menu(&mut state, true);
    let actual = state.source_menu.as_ref().unwrap();
    let expected = &fixture["layout"];
    assert_eq!(
        [actual.layout.size.0 as u32, actual.layout.size.1 as u32],
        [
            expected["size"][0].as_u64().unwrap() as u32,
            expected["size"][1].as_u64().unwrap() as u32
        ]
    );
    for (index, row) in actual.rows.iter().enumerate() {
        let expected = &expected["rows"][index];
        assert_eq!(row.label, expected["label"].as_str().unwrap());
        assert_eq!(row.shortcut, expected["shortcut"].as_str().unwrap());
        if !row.separator() {
            assert_eq!(row.enabled, expected["enabled"].as_bool().unwrap());
        }
        let rect = actual.layout.rows[index];
        assert_eq!(
            [
                rect.x() as i32,
                rect.y() as i32,
                rect.width() as i32,
                rect.height() as i32
            ],
            std::array::from_fn(|i| expected["rect"][i].as_i64().unwrap() as i32)
        );
    }
}

/// 【原文菜单测试】【输入隔离】菜单取消候选，拒绝正文键盘、鼠标和输入法；关闭后恢复原文身份，无参数、无返回值
#[test]
fn popup_suspends_composition_without_changing_selection_or_text() {
    let mut state = window();
    state.source.set_byte_selection(&mut state.fonts, 5, 5);
    state.event(WindowEvent::TextInput(Edit {
        preedit: Preedit {
            text: "ni".into(),
            cursor: Some((2, 2)),
        },
        commit: String::new(),
        replacement: None,
    }));
    assert!(state.source.preedit().is_some());
    let id = menu(&mut state, true);
    assert!(state.source.preedit().is_none());
    assert!(state.text_input_target().is_none());
    assert!(state.source.selected_text().is_none());
    let before = state.source.text();
    state.event(WindowEvent::Key(KeyInput {
        key: None,
        text: "intruder".into(),
        mods: Mods::default(),
    }));
    state.event(WindowEvent::TextInput(Edit {
        commit: "intruder".into(),
        preedit: Preedit::default(),
        replacement: None,
    }));
    state.event(WindowEvent::PointerMove {
        position: (200.0, 200.0),
    });
    assert_eq!(state.source.text(), before);
    assert!(state.source.selected_text().is_none());
    closed(&mut state, id, None);
    assert!(state.text_input_target().is_some());
}

/// 【原文菜单测试】【鼠标职责】右键不复制，左键按下不执行，释放只返回待交付动作；动作交付后菜单消失，无参数、无返回值
#[test]
fn menu_actions_wait_for_left_release_and_explicit_close() {
    let mut state = window();
    state.source.set_byte_selection(&mut state.fonts, 0, 5);
    let id = menu(&mut state, true);
    let point = state.source_menu.as_ref().unwrap().layout.rows[6];
    for (button, pressed, expected) in [
        (MouseButton::Right, true, WindowPopupAction::None),
        (MouseButton::Right, false, WindowPopupAction::None),
        (MouseButton::Left, true, WindowPopupAction::None),
        (MouseButton::Left, false, WindowPopupAction::Activate(6)),
    ] {
        assert_eq!(
            event(
                &mut state,
                id,
                PopupEvent::Button {
                    x: f64::from(point.x() + 20.0),
                    y: f64::from(point.y() + 10.0),
                    button,
                    pressed
                }
            ),
            expected
        );
        assert_eq!(state.source.text(), "alpha beta\ngamma");
        assert!(state.source_menu.is_some());
    }
    closed(&mut state, id, Some(6));
    assert!(state.source_menu.is_none());
    assert_eq!(state.source.text(), " beta\ngamma");
    assert!(state.source.can_undo());
    let id = menu(&mut state, true);
    closed(&mut state, id, Some(0));
    assert_eq!(state.source.text(), "alpha beta\ngamma");
    let id = menu(&mut state, true);
    closed(&mut state, id, Some(1));
    assert_eq!(state.source.text(), " beta\ngamma");
}

/// 【原文菜单测试】【禁用导航】跳过分隔和禁用行，首字母循环不执行动作，控制快捷键不编辑正文，无参数、无返回值
#[test]
fn navigation_and_mnemonics_skip_disabled_items() {
    let mut state = window();
    state.source.set_byte_selection(&mut state.fonts, 0, 5);
    let id = menu(&mut state, true);
    for (key, expected) in [
        (Key::Down, 3),
        (Key::Down, 4),
        (Key::Up, 3),
        (Key::Home, 3),
        (Key::End, 8),
        (Key::Tab, 3),
    ] {
        event(
            &mut state,
            id,
            PopupEvent::Key {
                chord: Some(Chord {
                    key,
                    mods: Mods::default(),
                }),
                text: None,
            },
        );
        assert_eq!(state.source_menu.as_ref().unwrap().active, Some(expected));
    }
    state.source_menu.as_mut().unwrap().active = None;
    for (letter, expected) in [('c', 3), ('c', 4), ('p', 5)] {
        event(
            &mut state,
            id,
            PopupEvent::Key {
                chord: Some(Chord {
                    key: Key::Char(letter),
                    mods: Mods::default(),
                }),
                text: Some(letter.to_string()),
            },
        );
        assert_eq!(state.source_menu.as_ref().unwrap().active, Some(expected));
    }
    event(
        &mut state,
        id,
        PopupEvent::Key {
            chord: Some(Chord {
                key: Key::Char('x'),
                mods: Mods {
                    ctrl: true,
                    ..Mods::default()
                },
            }),
            text: Some("x".into()),
        },
    );
    assert_eq!(state.source.text(), "alpha beta\ngamma");
    assert_eq!(state.source_menu.as_ref().unwrap().active, Some(5));
}

/// 【原文菜单测试】【过期事件】旧编号、禁用项和非正文页面不执行编辑，新菜单保持独立会话，无参数、无返回值
#[test]
fn stale_or_disabled_actions_never_change_a_new_session() {
    let mut state = window();
    let id = menu(&mut state, false);
    closed(&mut state, id + 1, Some(8));
    assert!(state.source_menu.is_some());
    closed(&mut state, id, Some(6));
    assert_eq!(state.source.text(), "alpha beta\ngamma");
    assert!(!state.source.can_undo());
    state.view = View::Image;
    state.event(WindowEvent::PointerButton {
        position: (120.0, 120.0),
        button: crate::backend::wayland::window::RIGHT_BUTTON,
        pressed: true,
    });
    assert!(state.source_menu.is_none());
}

/// 【原文菜单测试】【增量像素】一倍、分数和两倍菜单有独立像素且静止不重复绘制，配置不修改锚点，无参数、无返回值
#[test]
fn native_frames_are_cached_and_scale_independently() {
    let mut state = window();
    let id = menu(&mut state, true);
    for scale in [1.0, 1.2, 2.0] {
        let produced = frame(&mut state, scale).unwrap();
        assert_eq!(produced.id, id);
        assert_eq!(
            produced.pixels.len(),
            (produced.pixel_size.0 * produced.pixel_size.1 * 4) as usize
        );
        assert!(frame(&mut state, scale).is_none());
    }
    event(
        &mut state,
        id,
        PopupEvent::Configured { position: (10, 20) },
    );
    assert_eq!(state.source_menu.as_ref().unwrap().anchor, (120.0, 120.0));
}
