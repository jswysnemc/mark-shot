//! 【工具栏设置】【交互事务】按下只选择、释放才排序，失败不替换预览或原配置

use super::super::window::{Preferences, contains};
use super::{
    drag::{self, Drag, Source},
    layout::{Hit, Layout},
};
use crate::{
    backend::window::{LEFT_BUTTON, WindowAction, WindowEvent},
    config::toolbar_layout::ToolbarLayout,
    keys::Key,
    ui::toolbar::{ToolbarItem, catalog},
};
use serde_json::Value;

/// 【工具栏设置】【联合提交】将 config 两栏一次写入 state 配置，成功后更新 selected，失败保留原布局
pub(super) fn save_layout(
    state: &mut Preferences,
    config: ToolbarLayout,
    selected: Option<usize>,
) -> bool {
    let value = serde_json::to_value(&config).unwrap();
    let result = crate::config::preferences::write_fields(
        &state.path,
        "toolbar_layout",
        &[
            ("main", value["main"].clone()),
            ("actions", value["actions"].clone()),
        ],
    )
    .and_then(|()| crate::config::preferences::read(&state.path));
    match result {
        Ok(values) => {
            state.values = values;
            if let Err(error) = state.publish() {
                state.status = format!("已保存，应用失败 / Saved; unable to apply: {error}");
            } else {
                state.status = if state.chinese() {
                    "已保存，返回截图立即生效"
                } else {
                    "Saved. Applies on return."
                }
                .into();
            }
            state.toolbar.config = config;
            state.toolbar.selected = selected;
            state.toolbar.scroll = state.toolbar.scroll.min(Layout::new(state).maximum());
            true
        }
        Err(error) => {
            state.status = error.to_string();
            false
        }
    }
}

/// 【工具栏设置】【布局保存】把 items 保存到 state 当前场景，selected 指定完成后的选择，成功刷新配置并清理拖动
fn save(state: &mut Preferences, items: Vec<ToolbarItem>, selected: Option<usize>) {
    let value = items
        .iter()
        .map(|item| Value::from(catalog::id(*item)))
        .collect::<Vec<_>>();
    if state.save_field("toolbar_layout", state.toolbar.profile.key(), value.into()) {
        state.toolbar.config =
            serde_json::from_value(state.values["toolbar_layout"].clone()).unwrap();
        state.toolbar.selected = selected;
        state.toolbar.scroll = state.toolbar.scroll.min(Layout::new(state).maximum());
        state.status = if state.chinese() {
            "已保存，返回截图立即生效"
        } else {
            "Saved. Applies when returning to the editor."
        }
        .into();
    }
}

/// 【工具栏设置】【按钮执行】在 state 中执行 hit，所有动作保存到实际配置，无占位按钮
pub(super) fn activate(state: &mut Preferences, hit: Hit) {
    let mut items = state.toolbar.profile.items(&state.toolbar.config).to_vec();
    match hit {
        Hit::Orientation(profile) => {
            let key = if profile == crate::config::toolbar_layout::Profile::Actions {
                "actions_vertical"
            } else {
                "main_vertical"
            };
            let vertical = !profile.vertical(&state.toolbar.config);
            if state.save_field("toolbar_layout", key, vertical.into()) {
                state.toolbar.config =
                    serde_json::from_value(state.values["toolbar_layout"].clone()).unwrap();
            }
        }
        Hit::Profile(profile) => {
            state.toolbar.profile = profile;
            state.toolbar.selected = None;
            state.toolbar.scroll = state.toolbar.scroll.min(Layout::new(state).maximum());
        }
        Hit::Item(profile, index) => {
            state.toolbar.profile = profile;
            state.toolbar.selected = Some(index);
        }
        Hit::Add(item) => {
            if item != ToolbarItem::Seperator
                && (state.toolbar.config.main.contains(&item)
                    || state.toolbar.config.actions.contains(&item))
            {
                return;
            }
            let index = state
                .toolbar
                .selected
                .map_or(items.len(), |index| index + 1)
                .min(items.len());
            items.insert(index, item);
            save(state, items, Some(index));
        }
        Hit::Reset(profile) => {
            state.toolbar.profile = profile;
            state.toolbar.selected = None;
            let mut config = state.toolbar.config.clone();
            let restored = profile.items(&ToolbarLayout::default()).to_vec();
            let other = if profile == crate::config::toolbar_layout::Profile::Main {
                crate::config::toolbar_layout::Profile::Actions
            } else {
                crate::config::toolbar_layout::Profile::Main
            };
            super::drag::items(&mut config, other)
                .retain(|item| *item == ToolbarItem::Seperator || !restored.contains(item));
            *super::drag::items(&mut config, profile) = restored;
            save_layout(state, config, None);
        }
        Hit::Left | Hit::Right | Hit::Remove => {
            if let Some(index) = state.toolbar.selected.filter(|index| *index < items.len()) {
                if hit == Hit::Remove {
                    items.remove(index);
                    save(state, items, None);
                } else {
                    let next = if hit == Hit::Left {
                        index.saturating_sub(1)
                    } else {
                        (index + 1).min(items.len() - 1)
                    };
                    items.swap(index, next);
                    save(state, items, Some(next));
                }
            }
        }
        Hit::Shortcut => {
            state.toolbar.recording = super::shortcuts::key(state);
            state.status = if state.chinese() {
                "录入以添加绑定 · Delete 清除 · Backspace 恢复默认 · Esc 取消"
            } else {
                "Record to add · Delete clears · Backspace resets · Esc cancels"
            }
            .into();
        }
    }
}

/// 【工具栏设置】【原生事件】按 event 更新 state 页面手势，返回消费结果；导航和窗口控件仍交给父窗口
pub(in crate::features::preferences) fn event(
    state: &mut Preferences,
    event: &WindowEvent,
) -> Option<WindowAction> {
    if let WindowEvent::Key(input) = event
        && super::shortcuts::input(state, input)
    {
        return Some(WindowAction::Redraw);
    }
    let layout = Layout::new(state);
    match event {
        WindowEvent::Resize { .. } | WindowEvent::Focused(false) => {
            state.toolbar.cancel();
            return None;
        }
        WindowEvent::PointerMove { position } => {
            state.toolbar.keyboard = None;
            if let Some(drag) = state.toolbar.scrollbar_drag
                && let Some(bar) = layout.scrollbar
            {
                state.toolbar.scroll = drag.value(*position, &bar);
                return Some(WindowAction::Redraw);
            }
            if state.toolbar.drag.is_some() {
                drag::update(state, *position);
                return Some(WindowAction::Redraw);
            }
            if contains(layout.viewport, *position) || contains(layout.viewport, state.pointer) {
                let changed = layout.hit(state.pointer) != layout.hit(*position);
                state.pointer = *position;
                return Some(if changed {
                    WindowAction::Redraw
                } else {
                    WindowAction::None
                });
            }
            return None;
        }
        WindowEvent::PointerButton {
            position,
            button: LEFT_BUTTON,
            pressed,
        } => {
            state.pointer = *position;
            if !pressed && state.toolbar.scrollbar_drag.take().is_some() {
                return Some(WindowAction::Redraw);
            }
            if !pressed && state.toolbar.drag.is_some() && drag::finish(state, *position) {
                return Some(WindowAction::Redraw);
            }
            if *pressed
                && let Some(bar) = &layout.scrollbar
                && contains(bar.track, *position)
            {
                if contains(bar.handle, *position) {
                    state.toolbar.scrollbar_drag =
                        Some(crate::ui::scrollbar::Drag::new(*position, bar));
                } else if position.1 < f64::from(bar.handle.top()) {
                    state.toolbar.scroll = state.toolbar.scroll.saturating_sub(bar.page);
                } else {
                    state.toolbar.scroll = (state.toolbar.scroll + bar.page).min(bar.maximum);
                }
                return Some(WindowAction::Redraw);
            }
            if *pressed {
                state.toolbar.pressed = layout.hit(*position);
                if contains(layout.viewport, *position) {
                    state.search_focus = false;
                }
                if let Some(Hit::Item(profile, index)) = state.toolbar.pressed {
                    state.toolbar.profile = profile;
                    state.toolbar.selected = Some(index);
                    state.toolbar.drag = Some(Drag {
                        source: Source::Bar(profile, index),
                        start: *position,
                        target: None,
                        active: false,
                    });
                } else if let Some(Hit::Add(item)) = state.toolbar.pressed {
                    state.toolbar.drag = Some(Drag {
                        source: Source::Pool(item),
                        start: *position,
                        target: None,
                        active: false,
                    });
                }
            } else if let Some(hit) = state
                .toolbar
                .pressed
                .take()
                .filter(|hit| Some(*hit) == layout.hit(*position))
            {
                activate(state, hit);
            }
            if !contains(layout.viewport, *position) {
                return None;
            }
        }
        WindowEvent::Scroll { position, delta } if contains(layout.viewport, *position) => {
            state.toolbar.scroll = state
                .toolbar
                .scroll
                .saturating_add_signed((delta.1 * 4.0).round() as isize)
                .min(layout.maximum());
        }
        WindowEvent::Key(input) if !state.search_focus => {
            if input.key == Some(Key::Escape) && state.toolbar.drag.is_some() {
                state.toolbar.cancel();
            } else {
                match input.key {
                    Some(Key::Tab) => {
                        let controls: Vec<_> = layout
                            .controls
                            .iter()
                            .copied()
                            .chain(layout.bars.iter().flat_map(|bar| {
                                bar.cells
                                    .iter()
                                    .enumerate()
                                    .map(|(index, area)| (Hit::Item(bar.profile, index), *area))
                            }))
                            .chain(
                                layout
                                    .available
                                    .iter()
                                    .map(|(item, area)| (Hit::Add(*item), *area)),
                            )
                            .collect();
                        if !controls.is_empty() {
                            let current = state
                                .toolbar
                                .keyboard
                                .and_then(|hit| controls.iter().position(|entry| entry.0 == hit));
                            let next = current.map_or(
                                if input.mods.shift {
                                    controls.len() - 1
                                } else {
                                    0
                                },
                                |index| {
                                    (index
                                        + if input.mods.shift {
                                            controls.len() - 1
                                        } else {
                                            1
                                        })
                                        % controls.len()
                                },
                            );
                            state.toolbar.keyboard = Some(controls[next].0);
                            let area = controls[next].1;
                            let delta = if area.top() < layout.viewport.top() {
                                area.top() - layout.viewport.top()
                            } else {
                                (area.bottom() - layout.viewport.bottom()).max(0.0)
                            };
                            state.toolbar.scroll = state
                                .toolbar
                                .scroll
                                .saturating_add_signed(delta as isize)
                                .min(layout.maximum());
                        }
                    }
                    Some(Key::Enter) if state.toolbar.keyboard.is_some() => {
                        activate(state, state.toolbar.keyboard.unwrap())
                    }
                    Some(Key::Delete) => activate(state, Hit::Remove),
                    Some(Key::Left) if input.mods.alt => activate(state, Hit::Left),
                    Some(Key::Right) if input.mods.alt => activate(state, Hit::Right),
                    Some(Key::PageDown) => {
                        state.toolbar.scroll = (state.toolbar.scroll
                            + layout.viewport.height() as usize)
                            .min(layout.maximum())
                    }
                    Some(Key::PageUp) => {
                        state.toolbar.scroll = state
                            .toolbar
                            .scroll
                            .saturating_sub(layout.viewport.height() as usize)
                    }
                    Some(Key::Home) => state.toolbar.scroll = 0,
                    Some(Key::End) => state.toolbar.scroll = layout.maximum(),
                    _ => return None,
                }
            }
        }
        _ => return None,
    }
    Some(WindowAction::Redraw)
}
