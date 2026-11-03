//! 【截图历史测试】【窗口回归】私有状态中的焦点、按钮事务与滚动行为

use super::*;
use crate::{
    backend::wayland::window::{KeyInput, LEFT_BUTTON, RIGHT_BUTTON},
    keys::{Key, Mods},
};
/// 【截图历史测试】【窗口状态】创建使用私有目录的未连接窗口；返回窗口和保留目录生命周期的句柄
fn state() -> (HistoryWindow, tempfile::TempDir) {
    let directory = tempfile::tempdir().unwrap();
    let settings = History {
        directory: directory.path().join("history").to_string_lossy().into(),
        ..History::default()
    };
    (
        HistoryWindow::with_settings(settings, false).unwrap(),
        directory,
    )
}
/// 【截图历史测试】【按键构造】返回 key 与 mods 对应的窗口事件，不发送真实桌面输入
fn key(key: Key, mods: Mods) -> WindowEvent {
    WindowEvent::Key(KeyInput {
        key: Some(key),
        mods,
        text: String::new(),
    })
}

/// 【截图历史测试】【原版快捷键】Ctrl+F 不得开启原版不存在的搜索字段，样本来自真实 Qt 投递，无参数、无返回值
#[test]
fn qt_history_ctrl_f_does_not_open_an_extra_editor() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/qt-history-window.json"
    ))
    .unwrap();
    assert_eq!(reference["ctrl_f_changed_focus"], false);
    let (mut state, _directory) = state();
    state.focus = Focus::List;
    state.event(key(
        Key::Char('f'),
        Mods {
            ctrl: true,
            ..Mods::default()
        },
    ));
    assert!(
        state.focus == Focus::List,
        "Ctrl+F moved focus to an editor absent from the original history window"
    );
}

/// 【截图历史测试】【右键职责】右键仅选择列表记录，不触发编辑任务；样本来自真实 Qt 鼠标投递，无参数、无返回值
#[test]
fn qt_history_right_click_selects_a_record_without_editing() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/qt-history-window.json"
    ))
    .unwrap();
    assert_eq!(reference["right_click_selected"], 1);
    assert_eq!(reference["right_button_copied"], false);
    let (mut state, _directory) = state();
    let png = super::super::tests::png(2, 2);
    state.store.append(&png, 1.0, "", 50).unwrap();
    state.store.append(&png, 1.0, "", 50).unwrap();
    state.model.refresh(state.store.entries().unwrap());
    let selected = state.model.visible[1];
    let name = state.model.entries[selected].name.clone();
    state.event(WindowEvent::PointerButton {
        position: (
            state.layout.list.x() as f64 + 20.0,
            state.layout.list.y() as f64 + super::super::layout::ROW as f64 * 1.5,
        ),
        button: RIGHT_BUTTON,
        pressed: true,
    });
    assert_eq!(state.model.selected.as_deref(), Some(name.as_str()));
    assert!(!state.jobs.foreground());
    assert!(state.last_click.is_none());
    let copy = state
        .layout
        .buttons
        .iter()
        .find(|(control, _)| *control == Control::Copy)
        .unwrap()
        .1;
    assert_eq!(
        state.event(WindowEvent::PointerButton {
            position: (copy.x() as f64 + 2.0, copy.y() as f64 + 2.0),
            button: RIGHT_BUTTON,
            pressed: true,
        }),
        WindowAction::None
    );
    assert_eq!(state.model.selected.as_deref(), Some(name.as_str()));
}
/// 【截图历史测试】【清空确认】默认 Enter 取消，确认弹层不允许点击底层复制按钮
#[test]
fn clear_confirmation_defaults_to_cancel_and_blocks_click_through() {
    let (mut state, _directory) = state();
    state.model.refresh(vec![super::super::Entry {
        name: "capture-20261104000000000-11111111111111111111111111111111.png".into(),
        created: chrono::Utc::now(),
        width: 2,
        height: 2,
        bytes: 20,
        scale: 1.0,
        search: String::new(),
    }]);
    state.control(Control::Clear);
    assert!(state.clear_confirmation);
    state.event(WindowEvent::PointerButton {
        position: (30.0, 410.0),
        button: LEFT_BUTTON,
        pressed: true,
    });
    assert!(state.clear_confirmation);
    state.event(key(Key::Enter, Mods::default()));
    assert!(!state.clear_confirmation);
    assert!(!state.store.directory().exists());
}
/// 【截图历史测试】【取消窗口】没有额外搜索或数量事务，Escape 直接关闭且不创建存储，无参数、无返回值
#[test]
fn escape_closes_without_extra_field_transactions() {
    let (mut state, _directory) = state();
    assert_eq!(
        state.event(key(Key::Escape, Mods::default())),
        WindowAction::Close
    );
    assert!(!state.store.directory().exists());
}

/// 【截图历史测试】【鼠标构造】按 position 和 pressed 创建左键事件，返回事件，不发送真实桌面输入
fn pointer(position: (f64, f64), pressed: bool) -> WindowEvent {
    WindowEvent::PointerButton {
        position,
        button: LEFT_BUTTON,
        pressed,
    }
}

/// 【截图历史测试】【控件位置】返回 state 中 control 的中心，用于验证窗口真实命中路径
fn center(state: &HistoryWindow, control: Control) -> (f64, f64) {
    let rect = state
        .layout
        .buttons
        .iter()
        .find(|(kind, _)| *kind == control)
        .unwrap()
        .1;
    (
        (rect.x() + rect.width() / 2.0) as f64,
        (rect.y() + rect.height() / 2.0) as f64,
    )
}

/// 【截图历史测试】【记录准备】为 state 创建 count 条私有记录并缓存首项，返回活动身份
fn records(state: &mut HistoryWindow, count: usize) -> String {
    let png = super::super::tests::png(2, 2);
    for _ in 0..count {
        state.store.append(&png, 1.0, "", 50).unwrap();
    }
    state.model.refresh(state.store.entries().unwrap());
    let name = state.model.selected.clone().unwrap();
    state.model.cache(name.clone(), Pixmap::new(2, 2).unwrap());
    name
}

/// 【截图历史测试】【滚动输入】滑块拖到底再拖回，轨道点击翻页且不切换选择，无参数、无返回值
#[test]
fn scrollbar_pointer_events_scroll_without_selecting_a_row() {
    let (mut state, _directory) = state();
    let name = records(&mut state, 30);
    let bar =
        super::super::scrollbar::geometry(state.layout.list, 30, state.layout.rows(), 0).unwrap();
    let point = ((bar.handle.x() + 2.0) as f64, (bar.handle.y() + 7.0) as f64);
    state.event(pointer(point, true));
    assert!(
        state.scrollbar_drag.is_some(),
        "scrollbar press was treated as a row selection"
    );
    state.event(WindowEvent::PointerMove {
        position: (point.0, 1000.0),
    });
    assert_eq!(state.model.scroll, 30 - state.layout.rows());
    assert_eq!(state.model.selected.as_deref(), Some(name.as_str()));
    state.event(WindowEvent::PointerMove { position: point });
    assert_eq!(state.model.scroll, 0);
    state.event(pointer(point, false));
    assert!(state.scrollbar_drag.is_none());
    state.event(pointer((point.0, bar.track.bottom() as f64 - 5.0), true));
    assert_eq!(state.model.scroll, bar.page);
    assert_eq!(state.model.selected.as_deref(), Some(name.as_str()));
}

/// 【截图历史测试】【列表状态】同一项点击保留反馈，空区域仍取得列表焦点并打断双击，无参数、无返回值
#[test]
fn repeated_selection_keeps_status_and_empty_list_click_takes_focus() {
    let (mut state, _directory) = state();
    records(&mut state, 1);
    state.status = Some("Copied screenshot".into());
    let point = (
        state.layout.list.x() as f64 + 30.0,
        state.layout.list.y() as f64 + 20.0,
    );
    state.event(pointer(point, true));
    assert_eq!(state.status.as_deref(), Some("Copied screenshot"));
    assert!(state.last_click.is_some());
    state.focus = Focus::Control(Control::Close);
    state.event(pointer((point.0, point.1 + 150.0), true));
    assert_eq!(state.focus, Focus::List);
    assert!(state.last_click.is_none());
}

/// 【截图历史测试】【按键释放】初始关闭焦点与原版一致，Enter 不触发，Space 只在释放时执行，无参数、无返回值
#[test]
fn qt_history_initial_focus_and_space_release_match() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/qt-history-details.json"
    ))
    .unwrap();
    assert_eq!(reference["initial_focus"], "Close");
    assert_eq!(reference["enter_clicked"], false);
    assert_eq!(reference["space_clicked_on_press"], false);
    assert_eq!(reference["space_clicked_on_release"], true);
    let (mut state, _directory) = state();
    assert_eq!(state.focus, Focus::Control(Control::Close));
    assert_eq!(
        state.event(key(Key::Enter, Mods::default())),
        WindowAction::None
    );
    assert_eq!(
        state.event(key(Key::Space, Mods::default())),
        WindowAction::Redraw
    );
    assert_eq!(state.pressed, Some(Control::Close));
    assert_eq!(
        state.event(WindowEvent::KeyRelease(KeyInput {
            key: Some(Key::Space),
            mods: Mods::default(),
            text: String::new()
        })),
        WindowAction::Close
    );
}

/// 【截图历史测试】【按钮取消】按下不执行，外部释放取消；切换焦点或失焦后 Space 释放不能执行旧事务，无参数、无返回值
#[test]
fn button_release_outside_and_focus_changes_cancel_the_transaction() {
    let (mut state, _directory) = state();
    let point = center(&state, Control::Close);
    assert_eq!(state.event(pointer(point, true)), WindowAction::Redraw);
    assert_eq!(
        state.event(pointer((350.0, 250.0), false)),
        WindowAction::Redraw
    );
    assert_eq!(state.pressed, None);
    state.event(key(Key::Space, Mods::default()));
    state.event(key(Key::Tab, Mods::default()));
    let release = || {
        WindowEvent::KeyRelease(KeyInput {
            key: Some(Key::Space),
            mods: Mods::default(),
            text: String::new(),
        })
    };
    assert_eq!(state.event(release()), WindowAction::None);
    state.focus = Focus::Control(Control::Close);
    state.event(key(Key::Space, Mods::default()));
    state.event(WindowEvent::Focused(false));
    assert_eq!(state.event(release()), WindowAction::None);
    state.event(pointer(point, true));
    assert_eq!(state.event(pointer(point, false)), WindowAction::Close);
}

/// 【截图历史测试】【焦点顺序】从列表按 Tab 遍历原版冻结序列，禁用项不参与，无参数、无返回值
#[test]
fn qt_history_tab_order_skips_disabled_controls() {
    let (mut state, _directory) = state();
    records(&mut state, 1);
    state.focus = Focus::List;
    for control in [
        Control::Copy,
        Control::Edit,
        Control::Pin,
        Control::Save,
        Control::Delete,
        Control::Clear,
        Control::Enabled,
        Control::Close,
    ] {
        state.event(key(Key::Tab, Mods::default()));
        assert_eq!(state.focus, Focus::Control(control));
    }
    state.event(key(Key::Tab, Mods::default()));
    assert_eq!(state.focus, Focus::List);
    state.model.refresh(Vec::new());
    state.event(key(Key::Tab, Mods::default()));
    assert_eq!(state.focus, Focus::Control(Control::Enabled));
}
