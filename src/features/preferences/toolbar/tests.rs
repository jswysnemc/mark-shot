//! 【工具栏回归】【交互验证】通过真实窗口事件验证布局保存、拖放边界和快捷键冲突

use super::{
    Editor,
    input::activate,
    layout::{Hit, Layout},
};
use crate::{
    backend::window::{KeyInput, LEFT_BUTTON, WindowContent, WindowEvent},
    config::{
        preferences,
        toolbar_layout::{Profile, ToolbarLayout},
    },
    features::preferences::{catalog::Page, window::Preferences},
    keys::{Key, Mods},
    ui::toolbar::{ToolbarItem, catalog},
};
use serde_json::json;

/// 【工具栏回归】【独立配置】无参数，返回持有临时目录的页面，避免测试写入用户配置
fn fixture() -> (tempfile::TempDir, Preferences) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.toml");
    std::fs::write(&path, "# preserve\n[custom]\nkeep = 17\n").unwrap();
    let mut state = Preferences::new(false).unwrap();
    state.path = path;
    state.values = preferences::read(&state.path).unwrap();
    state.toolbar = Editor::new(&state.values);
    state.page = Page::Toolbar;
    (directory, state)
}

/// 【工具栏回归】【坐标取值】按 area 返回中心位置，供绘制与真实事件共用
fn center(area: tiny_skia::Rect) -> (f64, f64) {
    (
        f64::from(area.left() + area.width() / 2.0),
        f64::from(area.top() + area.height() / 2.0),
    )
}

/// 【工具栏回归】【鼠标输入】按 position 和 pressed 向 state 投递左键事件，无返回值
fn pointer(state: &mut Preferences, position: (f64, f64), pressed: bool) {
    state.event(WindowEvent::PointerButton {
        position,
        button: LEFT_BUTTON,
        pressed,
    });
}

/// 【工具栏回归】【点击控件】按 hit 在 state 当前可视布局中点击按钮，无返回值
fn click(state: &mut Preferences, hit: Hit) {
    let layout = Layout::new(state);
    let area = layout
        .controls
        .iter()
        .find(|(candidate, _)| *candidate == hit)
        .unwrap()
        .1;
    let position = center(area);
    assert_eq!(layout.hit(position), Some(hit));
    pointer(state, position, true);
    pointer(state, position, false);
}

/// 【工具栏回归】【增删持久化】无参数，增删、排序与方向按场景隔离，重新读取后仍保持用户配置
#[test]
fn layout_changes_survive_reload_and_preserve_other_profiles() {
    let (_directory, mut state) = fixture();
    let original = state.toolbar.config.clone();
    click(&mut state, Hit::Profile(Profile::Actions));
    let item = catalog::parse("magnifier").unwrap();
    assert!(!state.toolbar.config.actions.contains(&item));
    activate(&mut state, Hit::Add(item));
    activate(&mut state, Hit::Left);
    let selected = state.toolbar.selected.unwrap();
    assert_eq!(state.toolbar.config.actions[selected], item);
    click(&mut state, Hit::Direction);
    let restored = preferences::load(&state.path).unwrap().toolbar_layout;
    assert_eq!(restored, state.toolbar.config);
    assert_eq!(restored.main, original.main);
    assert_eq!(restored.fullscreen, original.fullscreen);
    assert!(!restored.actions_vertical);
    click(&mut state, Hit::Remove);
    assert!(!state.toolbar.config.actions.contains(&item));
    click(&mut state, Hit::Reset);
    assert_eq!(state.toolbar.config.actions, original.actions);
    let saved = std::fs::read_to_string(&state.path).unwrap();
    assert!(saved.contains("# preserve") && saved.contains("keep = 17"));
}

/// 【工具栏回归】【拖放事务】无参数，按下不保存，释放时排序，离开预览取消且清除按下状态
#[test]
fn drag_commits_on_release_and_cancels_outside_preview() {
    let (_directory, mut state) = fixture();
    let original = state.toolbar.config.fullscreen.clone();
    let layout = Layout::new(&state);
    let start = center(layout.cells[0]);
    let target = center(layout.cells[2]);
    pointer(&mut state, start, true);
    state.event(WindowEvent::PointerMove { position: target });
    assert_eq!(state.toolbar.config.fullscreen, original);
    pointer(&mut state, target, false);
    assert_eq!(state.toolbar.config.fullscreen[2], original[0]);
    assert_eq!(state.toolbar.selected, Some(2));
    assert!(state.toolbar.pressed.is_none());
    let saved = std::fs::read_to_string(&state.path).unwrap();
    pointer(&mut state, start, true);
    state.event(WindowEvent::PointerMove {
        position: (0.0, 0.0),
    });
    pointer(&mut state, (0.0, 0.0), false);
    assert_eq!(std::fs::read_to_string(&state.path).unwrap(), saved);
    assert!(state.toolbar.drag.is_none() && state.toolbar.pressed.is_none());
}

/// 【工具栏回归】【快捷键冲突】无参数，普通录入和恢复默认数组均拒绝冲突，并保留现有绑定
#[test]
fn shortcuts_record_clear_and_reject_conflicting_defaults() {
    let (_directory, mut state) = fixture();
    let copy = catalog::parse("copy").unwrap();
    state.toolbar.selected = state
        .toolbar
        .config
        .fullscreen
        .iter()
        .position(|item| *item == copy);
    assert!(state.toolbar.selected.is_some());
    click(&mut state, Hit::Shortcut);
    state.event(WindowEvent::Key(KeyInput {
        key: Some(Key::Char('s')),
        text: String::new(),
        mods: Mods {
            ctrl: true,
            ..Default::default()
        },
    }));
    assert!(state.toolbar.recording);
    assert_eq!(
        crate::features::preferences::shortcuts::text(&state.values, "copy"),
        "Ctrl+C"
    );
    state.event(WindowEvent::Key(KeyInput {
        key: Some(Key::Char('c')),
        text: String::new(),
        mods: Mods {
            ctrl: true,
            alt: true,
            ..Default::default()
        },
    }));
    assert!(!state.toolbar.recording);
    assert_eq!(
        preferences::read(&state.path).unwrap()["keys"]["copy"],
        "Ctrl+Alt+C"
    );
    click(&mut state, Hit::ClearShortcut);
    assert_eq!(state.values["keys"]["copy"], "");
    assert!(state.save_field("keys", "save", "Ctrl+C".into()));
    let before = std::fs::read_to_string(&state.path).unwrap();
    click(&mut state, Hit::ResetShortcut);
    assert_eq!(std::fs::read_to_string(&state.path).unwrap(), before);
    assert_eq!(state.values["keys"]["copy"], "");
    assert!(state.save_field("keys", "save", "Ctrl+S".into()));
    click(&mut state, Hit::ResetShortcut);
    assert_eq!(state.values["keys"]["copy"], json!(["Ctrl+C"]));
}

/// 【工具栏回归】【录入焦点】无参数，点击工具栏离开搜索焦点，Esc 取消录入而不关闭设置窗口
#[test]
fn toolbar_click_leaves_search_and_escape_cancels_recording() {
    let (_directory, mut state) = fixture();
    state.search_focus = true;
    let position = center(Layout::new(&state).cells[0]);
    pointer(&mut state, position, true);
    pointer(&mut state, position, false);
    assert!(!state.search_focus);
    click(&mut state, Hit::Shortcut);
    assert!(state.toolbar.recording);
    let before = std::fs::read_to_string(&state.path).unwrap();
    state.event(WindowEvent::Key(KeyInput {
        key: Some(Key::Escape),
        text: String::new(),
        mods: Default::default(),
    }));
    assert!(!state.toolbar.recording);
    assert_eq!(std::fs::read_to_string(&state.path).unwrap(), before);
}

/// 【工具栏回归】【窄窗适配】无参数，最小窗口预览换行且候选末项可通过滚动访问
#[test]
fn narrow_window_wraps_and_scrolls_to_the_last_candidate() {
    let (_directory, mut state) = fixture();
    state.event(WindowEvent::Resize { size: (580, 420) });
    let layout = Layout::new(&state);
    assert!(layout.cells.last().unwrap().top() > layout.cells[0].top());
    assert!(
        layout
            .cells
            .iter()
            .all(|cell| cell.left() >= layout.preview.left()
                && cell.right() <= layout.preview.right())
    );
    assert!(layout.scrollbar.is_some());
    state.event(WindowEvent::Key(KeyInput {
        key: Some(Key::End),
        text: String::new(),
        mods: Default::default(),
    }));
    let layout = Layout::new(&state);
    assert_eq!(state.toolbar.scroll, layout.maximum());
    let (item, area) = layout.available.last().unwrap();
    assert_eq!(*item, ToolbarItem::Seperator);
    assert_eq!(layout.hit(center(*area)), Some(Hit::Add(*item)));
    state.toolbar.prepare();
    let mut canvas = tiny_skia::Pixmap::new(580, 420).unwrap();
    state.draw(&mut canvas, 1.0);
}

/// 【工具栏回归】【非法布局】无参数，拒绝空栏、重复按钮和未知按钮，分隔线允许重复
#[test]
fn invalid_layouts_and_shortcut_arrays_leave_configuration_untouched() {
    let (_directory, state) = fixture();
    let original = std::fs::read_to_string(&state.path).unwrap();
    for value in [
        json!([]),
        json!(["separator"]),
        json!(["copy", "copy"]),
        json!(["unknown"]),
    ] {
        assert!(preferences::write(&state.path, "toolbar_layout", "main", value).is_err());
        assert_eq!(std::fs::read_to_string(&state.path).unwrap(), original);
    }
    assert!(
        preferences::write(
            &state.path,
            "keys",
            "copy",
            json!(["Ctrl+C", "invalid-key"])
        )
        .is_err()
    );
    assert_eq!(std::fs::read_to_string(&state.path).unwrap(), original);
    preferences::write(
        &state.path,
        "toolbar_layout",
        "main",
        json!(["copy", "separator", "separator"]),
    )
    .unwrap();
    let saved: ToolbarLayout = preferences::load(&state.path).unwrap().toolbar_layout;
    assert_eq!(saved.main.len(), 3);
}
