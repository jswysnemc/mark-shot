//! 【工具栏回归】【编辑器同步】验证全屏合并、返回双栏与配置更新

mod geometry;

use super::{configure, update_toolbar};
use crate::{
    app::input::tests::editor,
    config::toolbar_layout::{Profile, ToolbarLayout},
    ui::toolbar::{ToolbarItem, catalog},
};

/// 【工具栏回归】【配置列表】按 names 返回用于编辑器测试的按钮列表
fn items(names: &[&str]) -> Vec<ToolbarItem> {
    names
        .iter()
        .map(|name| catalog::parse(name).unwrap())
        .collect()
}

/// 【工具栏回归】【配置同步】无参数，全屏按两栏顺序合并且侧栏修改立即进入合并结果，返回后主侧栏独立
#[test]
fn fullscreen_uses_current_bars_and_returns_to_main() {
    let mut state = editor();
    let mut config = ToolbarLayout {
        main: items(&["pen", "copy"]),
        actions: items(&["copy", "undo"]),
        ..Default::default()
    };
    configure(&mut state, &config, true);
    assert_eq!(state.toolbar.items, config.main);
    assert_eq!(state.action_toolbar.items, config.actions);
    state.fullscreen_scope = true;
    configure(&mut state, &config, false);
    assert_eq!(state.toolbar.profile, Profile::Fullscreen);
    assert_eq!(
        state.toolbar.items,
        items(&["pen", "copy", "separator", "undo"])
    );
    config.actions = items(&["redo", "copy"]);
    configure(&mut state, &config, false);
    assert_eq!(
        state.toolbar.items,
        items(&["pen", "copy", "separator", "redo"])
    );
    state.fullscreen_scope = false;
    configure(&mut state, &config, false);
    assert_eq!(state.toolbar.profile, Profile::Main);
    assert_eq!(state.toolbar.items, config.main);
    assert_eq!(state.action_toolbar.items, config.actions);
}

/// 【工具栏回归】【侧栏显隐】无参数，全屏时只显示合并栏，退出全屏后恢复两栏
#[test]
fn fullscreen_hides_side_toolbar_and_selection_restores_it() {
    let mut state = editor();
    let mut dirty = 0;
    state.selection.zone = tiny_skia::Rect::from_xywh(60.0, 60.0, 320.0, 240.0);
    update_toolbar(&mut state, &mut dirty);
    assert_eq!(
        (state.toolbar.opacity, state.action_toolbar.opacity),
        (1.0, 1.0)
    );
    let main = state.toolbar.items.clone();
    let side = state.action_toolbar.items.clone();
    state.fullscreen_scope = true;
    update_toolbar(&mut state, &mut dirty);
    assert_eq!(
        (state.toolbar.opacity, state.action_toolbar.opacity),
        (1.0, 0.0)
    );
    assert!(
        main.iter()
            .chain(&side)
            .all(|item| state.toolbar.items.contains(item))
    );
    state.fullscreen_scope = false;
    update_toolbar(&mut state, &mut dirty);
    assert_eq!(
        (state.toolbar.opacity, state.action_toolbar.opacity),
        (1.0, 1.0)
    );
    assert_eq!(state.toolbar.items, main);
    assert_eq!(state.action_toolbar.items, side);
}
