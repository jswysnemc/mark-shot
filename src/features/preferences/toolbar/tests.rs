//! 【工具栏回归】【交互验证】通过真实窗口事件验证两栏保存、拖放边界和键盘排序

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
    keys::Key,
    ui::toolbar::ToolbarItem,
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
    state.toolbar_editor = true;
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

/// 【工具栏回归】【增删持久化】无参数，增删与排序按主侧栏隔离，重新读取后仍保持用户配置
#[test]
fn layout_changes_survive_reload_and_preserve_other_profiles() {
    let (_directory, mut state) = fixture();
    let original = state.toolbar.config.clone();
    click(&mut state, Hit::Profile(Profile::Actions));
    let item = Layout::new(&state)
        .available
        .iter()
        .map(|(item, _)| *item)
        .find(|item| *item != ToolbarItem::Seperator)
        .unwrap();
    assert!(!state.toolbar.config.actions.contains(&item));
    activate(&mut state, Hit::Add(item));
    activate(&mut state, Hit::Left);
    let selected = state.toolbar.selected.unwrap();
    assert_eq!(state.toolbar.config.actions[selected], item);
    let restored = preferences::load(&state.path).unwrap().toolbar_layout;
    assert_eq!(restored, state.toolbar.config);
    assert_eq!(restored.main, original.main);
    assert!(restored.actions_vertical);
    click(&mut state, Hit::Remove);
    assert!(!state.toolbar.config.actions.contains(&item));
    click(&mut state, Hit::Reset(Profile::Actions));
    assert_eq!(state.toolbar.config.actions, original.actions);
    let saved = std::fs::read_to_string(&state.path).unwrap();
    assert!(saved.contains("# preserve") && saved.contains("keep = 17"));
}

/// 【工具栏回归】【拖放事务】无参数，按下不保存，释放时排序，离开预览取消且清除按下状态
#[test]
fn drag_commits_on_release_and_cancels_outside_preview() {
    let (_directory, mut state) = fixture();
    let original = state.toolbar.config.main.clone();
    let layout = Layout::new(&state);
    let start = center(layout.bars[0].cells[0]);
    let target = center(layout.bars[0].cells[2]);
    pointer(&mut state, start, true);
    state.event(WindowEvent::PointerMove { position: target });
    assert_eq!(state.toolbar.config.main, original);
    pointer(&mut state, target, false);
    assert_eq!(state.toolbar.config.main[2], original[0]);
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

/// 【工具栏回归】【两栏选择】无参数，点击侧栏项目切换添加目标，移除和重置不会改写主栏与快捷键
#[test]
fn selecting_side_item_targets_side_and_preserves_shortcuts() {
    let (_directory, mut state) = fixture();
    let original = state.toolbar.config.clone();
    let keys = state.values["keys"].clone();
    let layout = Layout::new(&state);
    assert_eq!(layout.bars.len(), 2);
    assert!(!layout.controls.iter().any(|(hit, _)| *hit == Hit::Remove));
    let position = center(layout.bars[1].cells[0]);
    pointer(&mut state, position, true);
    pointer(&mut state, position, false);
    assert_eq!(state.toolbar.profile, Profile::Actions);
    assert_eq!(state.toolbar.selected, Some(0));
    let item = original.actions[0];
    state.event(WindowEvent::Key(KeyInput {
        key: Some(Key::Right),
        text: String::new(),
        mods: crate::keys::Mods {
            alt: true,
            ..Default::default()
        },
    }));
    assert_eq!(state.toolbar.config.actions[1], item);
    state.event(WindowEvent::Key(KeyInput {
        key: Some(Key::Delete),
        text: String::new(),
        mods: Default::default(),
    }));
    assert!(!state.toolbar.config.actions.contains(&item));
    assert_eq!(state.toolbar.config.main, original.main);
    assert_eq!(state.values["keys"], keys);
    click(&mut state, Hit::Reset(Profile::Actions));
    assert_eq!(state.toolbar.config, original);
}

/// 【工具栏回归】【拖动焦点】无参数，点击工具栏离开搜索焦点，Esc 取消拖动而不保存布局
#[test]
fn toolbar_click_leaves_search_and_escape_cancels_drag() {
    let (_directory, mut state) = fixture();
    state.search_focus = true;
    let position = center(Layout::new(&state).bars[0].cells[0]);
    pointer(&mut state, position, true);
    assert!(!state.search_focus);
    assert!(state.toolbar.drag.is_some());
    let before = std::fs::read_to_string(&state.path).unwrap();
    state.event(WindowEvent::Key(KeyInput {
        key: Some(Key::Escape),
        text: String::new(),
        mods: Default::default(),
    }));
    assert!(state.toolbar.drag.is_none());
    pointer(&mut state, position, false);
    assert_eq!(std::fs::read_to_string(&state.path).unwrap(), before);
}

/// 【工具栏回归】【窄窗适配】无参数，最小窗口预览换行且候选末项可通过滚动访问
#[test]
fn narrow_window_wraps_and_scrolls_to_the_last_candidate() {
    let (_directory, mut state) = fixture();
    state.event(WindowEvent::Resize { size: (580, 420) });
    let layout = Layout::new(&state);
    for bar in &layout.bars {
        assert!(bar.cells.last().unwrap().top() > bar.cells[0].top());
        assert!(
            bar.cells.iter().all(
                |cell| cell.left() >= bar.preview.left() && cell.right() <= bar.preview.right()
            )
        );
    }
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

/// 【工具栏回归】【跨栏移动】无参数，真实拖放一次保存两栏，工具池始终排除两栏已有工具
#[test]
fn cross_bar_drag_moves_without_pool_duplicates() {
    let (_directory, mut state) = fixture();
    let layout = Layout::new(&state);
    let item = state.toolbar.config.main[0];
    let start = center(layout.bars[0].cells[0]);
    let target = center(layout.bars[1].cells[0]);
    pointer(&mut state, start, true);
    state.event(WindowEvent::PointerMove { position: target });
    assert!(state.toolbar.animation.deadline().is_some());
    pointer(&mut state, target, false);
    assert!(!state.toolbar.config.main.contains(&item));
    assert!(state.toolbar.config.actions.contains(&item));
    assert_eq!(
        preferences::load(&state.path).unwrap().toolbar_layout,
        state.toolbar.config
    );
    let layout = Layout::new(&state);
    for (item, _) in layout.available {
        if item != ToolbarItem::Seperator {
            assert!(!state.toolbar.config.main.contains(&item));
            assert!(!state.toolbar.config.actions.contains(&item));
        }
    }
}

/// 【工具栏回归】【空栏回滚】无参数，移走某栏最后一项失败时，两栏、当前目标和配置文件都保持原样
#[test]
fn moving_last_item_rolls_back_both_bars() {
    let (_directory, mut state) = fixture();
    state.toolbar.config.main.truncate(1);
    state.toolbar.profile = Profile::Main;
    let original = state.toolbar.config.clone();
    let text = std::fs::read_to_string(&state.path).unwrap();
    let layout = Layout::new(&state);
    let start = center(layout.bars[0].cells[0]);
    let target = center(layout.bars[1].cells[0]);
    pointer(&mut state, start, true);
    state.event(WindowEvent::PointerMove { position: target });
    pointer(&mut state, target, false);
    assert_eq!(state.toolbar.config, original);
    assert_eq!(state.toolbar.profile, Profile::Main);
    assert_eq!(std::fs::read_to_string(&state.path).unwrap(), text);
}
