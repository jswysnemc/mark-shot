//! 【截图工具栏】【定位同步】主栏跟随选区下缘，副栏优先避让到右侧，空选区不显示

use crate::{
    editor::{EditorState, dirty::mark_all_dirty},
    ui::{
        panel::{UiPanel, sync_panel_hover, sync_panel_rect},
        toolbar::{Toolbar, ToolbarButton, ToolbarItem},
    },
};
use tiny_skia::Rect;

#[cfg(test)]
mod tests;

/// 【截图工具栏】【状态刷新】按 state 的选区、手势和显示器更新双工具栏；dirty 返回重绘显示器
pub fn update_toolbar(state: &mut EditorState, dirty: &mut u32) {
    let config = crate::config::preferences::current();
    configure(state, &config.toolbar_layout, false);
    let dock = super::toolbar_docking::prepare(state, config.capture.toolbar_position);
    let docked = dock != crate::config::capture::ToolbarPosition::Selection;
    let monitor = if state.toolbar.user_position.is_some() || docked {
        state.toolbar.monitor_idx
    } else {
        state.input.pointer.monitor_idx
    };
    let bounds = state.placements[monitor].size;
    let available = (bounds.0 as f32 - 16.0, bounds.1 as f32 - 16.0);
    let hidden = state.selection.zone.is_none() || state.tool_active || state.ui_hidden;
    let old_main = state.toolbar.rect();
    let old_main_monitor = state.toolbar.monitor_idx;
    let old_side = state.action_toolbar.rect();
    let old_side_monitor = state.action_toolbar.monitor_idx;
    if state.toolbar.marker_shape != state.tool_settings.marker_shape
        || state.toolbar.marker_filled != state.tool_settings.marker_filled
    {
        state.toolbar.marker_shape = state.tool_settings.marker_shape;
        state.toolbar.marker_filled = state.tool_settings.marker_filled;
        state.toolbar.dirty = true;
    }
    let main_layout = state.toolbar.arrange(available);
    let side_layout = state.action_toolbar.arrange(available);
    let selection = state.selection.zone.and_then(|rect| {
        state
            .canvas_local_rect(rect, monitor)
            .and_then(|rect| rect.intersect(&state.placements[monitor].rect()?))
    });
    let dock_positions = docked.then(|| {
        super::toolbar_docking::positions(
            dock,
            (bounds.0 as f32, bounds.1 as f32),
            state.toolbar.size,
            (!state.fullscreen_scope).then_some(state.action_toolbar.size),
        )
    });
    let main_position = dock_positions
        .map(|positions| positions.0)
        .or(state.toolbar.user_position)
        .unwrap_or_else(|| {
            if state.fullscreen_scope && state.file_canvas.is_some() {
                return ((bounds.0 as f32 - state.toolbar.size.0) / 2.0, 12.0);
            }
            let rect = selection.unwrap_or_else(|| {
                Rect::from_xywh(0.0, 0.0, bounds.0 as f32, bounds.1 as f32).unwrap()
            });
            let x = (rect.left() + rect.width() / 2.0 - state.toolbar.size.0 / 2.0)
                .clamp(8.0, (bounds.0 as f32 - state.toolbar.size.0 - 8.0).max(8.0));
            let below = rect.bottom() + 8.0;
            let y = if below + state.toolbar.size.1 <= bounds.1 as f32 - 8.0 {
                below
            } else {
                rect.top() - state.toolbar.size.1 - 8.0
            };
            (
                x,
                y.clamp(8.0, (bounds.1 as f32 - state.toolbar.size.1 - 8.0).max(8.0)),
            )
        });
    place(&mut state.toolbar, monitor, main_position, hidden);
    let side_monitor = if state.action_toolbar.user_position.is_some() {
        state.action_toolbar.monitor_idx
    } else {
        monitor
    };
    let side_position = dock_positions
        .map(|positions| positions.1)
        .or(state.action_toolbar.user_position)
        .unwrap_or_else(|| {
            let rect = selection.unwrap_or_else(|| {
                Rect::from_xywh(0.0, 0.0, bounds.0 as f32, bounds.1 as f32).unwrap()
            });
            side_position(
                rect,
                state.toolbar.rect(),
                state.action_toolbar.size,
                (bounds.0 as f32, bounds.1 as f32),
            )
        });
    place(
        &mut state.action_toolbar,
        side_monitor,
        side_position,
        hidden || state.fullscreen_scope,
    );
    let undo = !state.undo_stack.is_empty();
    let redo = !state.redo_stack.is_empty();
    let clear =
        !state.annotations.is_empty() || state.text_session.is_some() || state.pending.is_some();
    let busy = state.ann_drag.is_some()
        || state.input.mouse_down
        || state.upload.busy()
        || state.selection_translation.busy();
    let translating = state.selection_translation.session.is_some()
        || state.selection_translation.popup.is_some();
    for toolbar in [&mut state.toolbar, &mut state.action_toolbar] {
        if toolbar.translating != translating {
            toolbar.translating = translating;
            toolbar.dirty = true;
        }
    }
    disable(
        &mut state.toolbar,
        undo || state.pending.is_some(),
        redo,
        clear,
        busy,
    );
    disable(
        &mut state.action_toolbar,
        undo || state.pending.is_some(),
        redo,
        clear,
        busy,
    );
    let main_changed = main_layout || state.toolbar.dirty;
    let side_changed = side_layout || state.action_toolbar.dirty;
    sync_panel_rect(
        &mut state.toolbar,
        old_main,
        old_main_monitor,
        main_changed,
        &mut state.damage_rects,
        dirty,
    );
    sync_panel_rect(
        &mut state.action_toolbar,
        old_side,
        old_side_monitor,
        side_changed,
        &mut state.damage_rects,
        dirty,
    );
    for toolbar in [&mut state.toolbar, &mut state.action_toolbar] {
        let hovered = if toolbar.monitor_idx == state.input.pointer.monitor_idx {
            toolbar.hit_test(state.input.pointer.local).1
        } else {
            None
        };
        sync_panel_hover(toolbar, hovered, &mut state.damage_rects, dirty);
    }
    if main_layout || side_layout {
        mark_all_dirty(dirty, state.placements.len());
    }
}

/// 【工具栏配置】【会话同步】按 config 应用 state 当前场景按钮，reset 清除旧位置并应用方向，不修改标注
pub(crate) fn configure(
    state: &mut EditorState,
    config: &crate::config::toolbar_layout::ToolbarLayout,
    reset: bool,
) {
    use crate::config::toolbar_layout::Profile;
    let profile = if state.fullscreen_scope {
        Profile::Fullscreen
    } else {
        Profile::Main
    };
    for (toolbar, profile) in [
        (&mut state.toolbar, profile),
        (&mut state.action_toolbar, Profile::Actions),
    ] {
        let items = profile.items(config);
        if toolbar.items != *items || toolbar.profile != profile || reset {
            toolbar.items = items.to_vec();
            toolbar.profile = profile;
            toolbar.orientation = if profile.vertical(config) {
                crate::ui::toolbar::Orientation::Vertical
            } else {
                crate::ui::toolbar::Orientation::Horizontal
            };
            toolbar.selected = items.iter().position(|item| matches!(item, ToolbarItem::Button(ToolbarButton::Tool(tool)) if *tool == state.selected_tool));
            toolbar.hovered = None;
            toolbar.disabled.clear();
            toolbar.user_position = None;
            if reset {
                toolbar.dock_override = None;
                toolbar.dock_monitor = None;
            }
            toolbar.drawn_look = None;
            toolbar.dirty = true;
        }
    }
}

/// 【截图工具栏】【可用状态】按 undo、redo、clear 和 busy 更新 toolbar 按钮，仅未实现集成入口保持禁用
fn disable(toolbar: &mut Toolbar, undo: bool, redo: bool, clear: bool, busy: bool) {
    let disabled = toolbar
        .items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            let disabled = match item {
                ToolbarItem::Button(ToolbarButton::Integration(integration)) => {
                    busy || !matches!(
                        integration,
                        crate::ui::toolbar::Integration::OpenWith
                            | crate::ui::toolbar::Integration::Extensions
                            | crate::ui::toolbar::Integration::Upload
                            | crate::ui::toolbar::Integration::Translate
                            | crate::ui::toolbar::Integration::Settings
                    )
                }
                ToolbarItem::Button(ToolbarButton::Undo) => !undo || busy,
                ToolbarItem::Button(ToolbarButton::Redo) => !redo || busy,
                ToolbarItem::Button(ToolbarButton::Clear) => !clear || busy,
                _ => busy,
            };
            disabled.then_some(index)
        })
        .collect::<Vec<_>>();
    if toolbar.disabled != disabled {
        toolbar.disabled = disabled;
        toolbar.dirty = true;
    }
}

/// 【截图工具栏】【位置设置】同步 toolbar 的 monitor、position、hidden；不保留跨屏移动动画
fn place(toolbar: &mut Toolbar, monitor: usize, position: (f32, f32), hidden: bool) {
    if toolbar.monitor_idx != monitor
        || toolbar.position != position
        || toolbar.interferes != hidden
    {
        toolbar.dirty = true;
    }
    toolbar.monitor_idx = monitor;
    toolbar.position = position;
    toolbar.render_pos = position;
    toolbar.interferes = hidden;
    // 1. 【截图工具栏】【显隐一致】原版选区完成才显示，隐藏时不留可命中的透明按钮
    toolbar.opacity = if hidden { 0.0 } else { 1.0 };
}

/// 【截图工具栏】【副栏避让】按 selection、main、size 和 bounds 返回原版右侧优先的安全位置
fn side_position(
    selection: Rect,
    main: Option<Rect>,
    size: (f32, f32),
    bounds: (f32, f32),
) -> (f32, f32) {
    let clamp = |x: f32, y: f32| {
        (
            x.clamp(8.0, (bounds.0 - size.0 - 8.0).max(8.0)),
            y.clamp(8.0, (bounds.1 - size.1 - 8.0).max(8.0)),
        )
    };
    let center_y = selection.top() + selection.height() / 2.0 - size.1 / 2.0;
    let mut candidates = vec![
        clamp(selection.right() + 8.0, center_y),
        clamp(selection.left() - size.0 - 8.0, center_y),
    ];
    if let Some(main) = main {
        candidates.extend([
            clamp(main.right() + 8.0, main.top()),
            clamp(main.left() - size.0 - 8.0, main.top()),
        ]);
    }
    candidates
        .iter()
        .copied()
        .find(|&(x, y)| {
            let rect = Rect::from_xywh(x - 4.0, y - 4.0, size.0 + 8.0, size.1 + 8.0).unwrap();
            !crate::utils::rects_overlap(&rect, &selection)
                && main.is_none_or(|main| !crate::utils::rects_overlap(&rect, &main))
        })
        .unwrap_or(candidates[0])
}
