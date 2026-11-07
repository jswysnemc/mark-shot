use super::*;
use crate::{
    backend::window::{WindowContent, WindowPopupAction},
    features::ocr_result::{document::OcrDocument, layout::Control},
    keys::{Chord, Key, Mods},
    types::{MouseButton, popup::PopupEvent},
};

/// 【更多菜单测试】【独立窗口】创建离线原文状态并打开真实父菜单，返回状态，不写配置、不连接桌面
fn state() -> ResultWindow {
    let mut state = ResultWindow::new(OcrDocument {
        text: "original".into(),
        png: crate::utils::encode_png(&tiny_skia::Pixmap::new(2, 2).unwrap()),
        lines: vec![],
    })
    .unwrap();
    state.control(Control::More);
    let root = state.popup_id().unwrap();
    event(
        &mut state,
        root,
        PopupEvent::Configured {
            position: (400, 36),
        },
    );
    state
}

/// 【更多菜单测试】【方向按键】按 key 创建真实菜单契约事件，返回事件，不生成正文文字
fn key(key: Key) -> PopupEvent {
    PopupEvent::Key {
        chord: Some(Chord {
            key,
            mods: Mods::default(),
        }),
        text: None,
    }
}

/// 【更多菜单测试】【原版几何】核对完整 Qt 窗口的真实父子行、快捷键及字体高度，无参数、无返回值
#[test]
fn menus_match_the_actual_qt_window_record() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/qt-ocr-more-menu.json"
    ))
    .unwrap();
    let mut state = state();
    let root = state.popup_id().unwrap();
    event(&mut state, root, key(Key::Down));
    event(&mut state, root, key(Key::Right));
    for (name, menu) in [
        ("root", state.source_menu.as_ref().unwrap()),
        ("child", state.more_menu.child.as_ref().unwrap()),
    ] {
        let expected = &fixture[name];
        assert_eq!(
            serde_json::json!([menu.layout.size.0 as i32, menu.layout.size.1 as i32]),
            expected["size"]
        );
        assert_eq!(
            menu.layout.line_height as i64,
            expected["font_height"].as_i64().unwrap()
        );
        assert_eq!(menu.rows.len(), expected["rows"].as_array().unwrap().len());
        for (index, row) in menu.rows.iter().enumerate() {
            let expected = &expected["rows"][index];
            assert_eq!(row.label, expected["label"].as_str().unwrap());
            assert_eq!(row.shortcut, expected["shortcut"].as_str().unwrap());
            assert_eq!(row.enabled, expected["enabled"].as_bool().unwrap());
            assert_eq!(
                menu.submenu_rows.contains(&index),
                expected["submenu"].as_bool().unwrap()
            );
            let rect = menu.layout.rows[index];
            assert_eq!(
                serde_json::json!([
                    rect.x() as i32,
                    rect.y() as i32,
                    rect.width() as i32,
                    rect.height() as i32
                ]),
                expected["rect"]
            );
        }
    }
}

/// 【更多菜单测试】【父子导航】右方向键建立独立子节点，全部说明禁用，左键和 Escape 逐层关闭，无参数、无返回值
#[test]
fn keyboard_opens_a_real_child_and_closes_only_the_deepest_level() {
    let mut state = state();
    let root = state.popup_id().unwrap();
    event(&mut state, root, key(Key::Down));
    event(&mut state, root, key(Key::Right));
    let nodes = state.popup_nodes();
    assert_eq!(nodes.len(), 2);
    assert_eq!(nodes[1].parent, Some(root));
    assert!(
        state
            .more_menu
            .child
            .as_ref()
            .unwrap()
            .rows
            .iter()
            .all(|row| !row.enabled)
    );
    for key_value in [Key::Down, Key::Up, Key::Enter, Key::Space, Key::Tab] {
        assert_eq!(
            event(&mut state, root, key(key_value)),
            WindowPopupAction::None
        );
    }
    assert_eq!(state.source.text(), "original");
    event(&mut state, root, key(Key::Left));
    assert_eq!(state.popup_nodes().len(), 1);
    assert_eq!(
        event(&mut state, root, key(Key::Left)),
        WindowPopupAction::None
    );
    assert_eq!(
        event(&mut state, root, key(Key::Escape)),
        WindowPopupAction::CloseAll
    );
}

/// 【更多菜单测试】【单次悬停】180 毫秒只展开一次，重复悬停不延长期限，进入空白可取消未执行展开，无参数、无返回值
#[test]
fn hover_is_a_single_consumed_deadline_and_can_be_canceled() {
    let mut state = state();
    let root = state.popup_id().unwrap();
    event(&mut state, root, PopupEvent::Pointer { x: 20.0, y: 15.0 });
    let deadline = state.next_deadline().unwrap();
    event(&mut state, root, PopupEvent::Pointer { x: 21.0, y: 16.0 });
    assert_eq!(state.next_deadline(), Some(deadline));
    assert!(!wake(&mut state, deadline - Duration::from_millis(1)));
    assert!(wake(&mut state, deadline));
    assert_eq!(state.popup_nodes().len(), 2);
    assert_eq!(state.next_deadline(), None);
    assert!(!wake(&mut state, deadline + Duration::from_secs(1)));
    state.more_menu.close_child(state.popup_nodes()[1].id);
    event(&mut state, root, PopupEvent::Pointer { x: 20.0, y: 15.0 });
    event(&mut state, root, PopupEvent::Pointer { x: 1.0, y: 1.0 });
    assert_eq!(state.next_deadline(), None);
}

/// 【更多菜单测试】【离开取消】真实表面离开和整树外坐标均取消未执行展开，失效期限不得创建子层，无参数、无返回值
#[test]
fn leaving_the_root_cancels_its_unconsumed_hover_deadline() {
    for leave in [
        PopupEvent::Leave,
        PopupEvent::Pointer { x: -20.0, y: -20.0 },
    ] {
        let mut state = state();
        let root = state.popup_id().unwrap();
        event(&mut state, root, PopupEvent::Pointer { x: 20.0, y: 15.0 });
        let deadline = state.next_deadline().unwrap();
        event(&mut state, root, leave);
        assert_eq!(state.next_deadline(), None);
        assert!(!wake(&mut state, deadline));
        assert!(state.more_menu.child.is_none());
        assert_eq!(state.source_menu.as_ref().unwrap().active, None);
    }
}

/// 【更多菜单测试】【按下展开】父行左键按下即展开，右键不直接展开，整树之外的按下关闭所有层，无参数、无返回值
#[test]
fn parent_press_and_outside_click_follow_the_original_menu_roles() {
    let mut state = state();
    let root = state.popup_id().unwrap();
    event(
        &mut state,
        root,
        PopupEvent::Button {
            x: 20.0,
            y: 15.0,
            button: MouseButton::Right,
            pressed: true,
        },
    );
    assert!(state.more_menu.child.is_none());
    event(
        &mut state,
        root,
        PopupEvent::Button {
            x: 20.0,
            y: 15.0,
            button: MouseButton::Left,
            pressed: true,
        },
    );
    assert_eq!(state.popup_nodes().len(), 2);
    assert_eq!(
        event(
            &mut state,
            root,
            PopupEvent::Button {
                x: -100.0,
                y: -100.0,
                button: MouseButton::Left,
                pressed: true
            }
        ),
        WindowPopupAction::CloseAll
    );
}

/// 【更多菜单测试】【过期帧隔离】父子像素使用不同编号与缓冲，旧子菜单输入不关闭重新打开的菜单，无参数、无返回值
#[test]
fn native_frames_and_stale_child_ids_are_isolated() {
    let mut state = state();
    let root = state.popup_id().unwrap();
    event(&mut state, root, key(Key::Down));
    event(&mut state, root, key(Key::Right));
    let child = state.popup_nodes()[1].id;
    assert_eq!(state.popup_frame_for(root, 1.2).unwrap().id, root);
    assert_eq!(state.popup_frame_for(child, 1.2).unwrap().id, child);
    assert!(state.popup_frame_for(child, 1.2).is_none());
    event(&mut state, child, key(Key::Left));
    event(&mut state, root, key(Key::Right));
    let next = state.popup_nodes()[1].id;
    assert_ne!(next, child);
    assert_eq!(
        event(&mut state, child, key(Key::Escape)),
        WindowPopupAction::None
    );
    assert_eq!(state.popup_nodes()[1].id, next);
}

/// 【更多菜单测试】【主题失效】尺寸与内容都未改变时，父子菜单旧主题仍必须生成新帧，并保留原生表面身份，无参数、无返回值
#[test]
fn cached_parent_and_child_menus_emit_frames_when_the_theme_changes() {
    let mut state = state();
    let root = state.popup_id().unwrap();
    event(&mut state, root, key(Key::Down));
    event(&mut state, root, key(Key::Right));
    let child = state.popup_nodes()[1].id;
    state.popup_frame_for(root, 1.2).unwrap();
    state.popup_frame_for(child, 1.2).unwrap();
    assert!(state.popup_frame_for(root, 1.2).is_none());
    assert!(state.popup_frame_for(child, 1.2).is_none());
    let previous = crate::theme::Mode::current().opposite();
    state.source_menu.as_mut().unwrap().drawn_theme = Some(previous);
    state.more_menu.child.as_mut().unwrap().drawn_theme = Some(previous);
    assert_eq!(state.popup_frame_for(root, 1.2).unwrap().id, root);
    assert_eq!(state.popup_frame_for(child, 1.2).unwrap().id, child);
    assert!(state.popup_frame_for(root, 1.2).is_none());
    assert!(state.popup_frame_for(child, 1.2).is_none());
}
