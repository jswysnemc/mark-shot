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

/// 【工具栏回归】【悬停提示】无参数，悬停超过延迟后显示气泡，离开按钮立即隐藏
#[test]
fn tooltip_shows_after_delay_and_hides_on_leave() {
    use std::time::{Duration, Instant};

    let mut state = editor();
    let mut dirty = 0;
    state.selection.zone = tiny_skia::Rect::from_xywh(60.0, 60.0, 320.0, 240.0);
    update_toolbar(&mut state, &mut dirty);

    // point at the center of the first toolbar button
    let (x, y) = {
        let toolbar = &state.toolbar;
        let cell = toolbar.cell(0).unwrap();
        (
            f64::from(toolbar.render_pos.0 + cell.left() + cell.width() / 2.0),
            f64::from(toolbar.render_pos.1 + cell.top() + cell.height() / 2.0),
        )
    };
    state.input.pointer = crate::types::PointerState::new(0, (x, y), (x, y));
    update_toolbar(&mut state, &mut dirty);
    assert_eq!(state.toolbar.hovered, Some(0));
    assert!(state.tooltip.is_none());

    // inside the delay nothing shows yet
    super::sync_tooltips(&mut state, &mut dirty);
    assert!(state.tooltip.is_none());

    // age the hover past the delay
    state.toolbar.hover_started = Some(
        Instant::now() - crate::ui::toolbar::tooltip::TOOLTIP_DELAY - Duration::from_millis(50),
    );
    super::sync_tooltips(&mut state, &mut dirty);
    let tip = state.tooltip.as_ref().expect("tooltip after delay");
    assert_eq!(
        (tip.owner_actions, tip.index, tip.monitor_idx),
        (false, 0, 0)
    );
    let cn = crate::ui::language::Language::current() == crate::ui::language::Language::Chinese;
    let label = crate::ui::toolbar::catalog::entry(state.toolbar.items[0])
        .unwrap()
        .label(cn);
    assert!(tip.text.starts_with(label));
    let bounds = state.placements[0].size;
    assert!(tip.rect.left() >= 0.0 && tip.rect.right() <= bounds.0 as f32);
    assert!(tip.rect.top() >= 0.0 && tip.rect.bottom() <= bounds.1 as f32);

    // leaving the button hides it immediately
    state.input.pointer = crate::types::PointerState::new(0, (4.0, 4.0), (4.0, 4.0));
    update_toolbar(&mut state, &mut dirty);
    assert_eq!(state.toolbar.hovered, None);
    assert!(state.tooltip.is_none());
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
