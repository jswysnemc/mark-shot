//! 【工具栏设置】【交互事务】按下只选择、释放才排序，失败不替换预览或原配置

use super::super::{
    shortcuts,
    window::{Preferences, contains},
};
use super::{
    Drag,
    layout::{Hit, Layout},
};
use crate::{
    backend::window::{LEFT_BUTTON, WindowAction, WindowEvent},
    config::toolbar_layout::ToolbarLayout,
    keys::Key,
    ui::toolbar::{ToolbarItem, catalog},
};
use serde_json::Value;

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

/// 【工具栏设置】【绑定键名】按 state 当前选择返回对应的全局动作键，分隔线或没有选择返回空值
pub(super) fn shortcut_key(state: &Preferences) -> Option<&'static str> {
    let item = *state
        .toolbar
        .profile
        .items(&state.toolbar.config)
        .get(state.toolbar.selected?)?;
    catalog::entry(item)?.shortcut()
}

/// 【工具栏设置】【按钮执行】在 state 中执行 hit，所有动作保存到实际配置，无占位按钮
pub(super) fn activate(state: &mut Preferences, hit: Hit) {
    state.toolbar.recording = false;
    let mut items = state.toolbar.profile.items(&state.toolbar.config).to_vec();
    match hit {
        Hit::Profile(profile) => {
            state.toolbar.profile = profile;
            state.toolbar.selected = None;
            state.toolbar.scroll = 0;
        }
        Hit::Item(index) => state.toolbar.selected = Some(index),
        Hit::Add(item) => {
            let index = state
                .toolbar
                .selected
                .map_or(items.len(), |index| index + 1)
                .min(items.len());
            items.insert(index, item);
            save(state, items, Some(index));
        }
        Hit::Reset => save(
            state,
            state
                .toolbar
                .profile
                .items(&ToolbarLayout::default())
                .to_vec(),
            None,
        ),
        Hit::Direction => {
            let profile = state.toolbar.profile;
            if state.save_field(
                "toolbar_layout",
                profile.direction_key(),
                (!profile.vertical(&state.toolbar.config)).into(),
            ) {
                state.toolbar.config =
                    serde_json::from_value(state.values["toolbar_layout"].clone()).unwrap();
            }
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
        Hit::Shortcut => state.toolbar.recording = shortcut_key(state).is_some(),
        Hit::ClearShortcut | Hit::ResetShortcut => {
            if let Some(key) = shortcut_key(state) {
                let value = if hit == Hit::ClearShortcut {
                    Value::String(String::new())
                } else {
                    Value::Array(shortcuts::defaults(key))
                };
                state.save_field("keys", key, value);
            }
        }
    }
}

/// 【工具栏设置】【原生事件】按 event 更新 state 页面手势，返回消费结果；导航和窗口控件仍交给父窗口
pub(in crate::features::preferences) fn event(
    state: &mut Preferences,
    event: &WindowEvent,
) -> Option<WindowAction> {
    let layout = Layout::new(state);
    match event {
        WindowEvent::Resize { .. } | WindowEvent::Focused(false) => {
            state.toolbar.cancel();
            return None;
        }
        WindowEvent::PointerMove { position } => {
            if let Some(drag) = state.toolbar.scrollbar_drag
                && let Some(bar) = layout.scrollbar
            {
                state.toolbar.scroll = drag.value(*position, &bar);
                return Some(WindowAction::Redraw);
            }
            if let Some(drag) = &mut state.toolbar.drag {
                drag.active |= (position.0 - drag.start.0).hypot(position.1 - drag.start.1) >= 4.0;
                drag.target = layout.insertion(*position).unwrap_or(drag.source);
                state.pointer = *position;
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
            if !pressed && let Some(drag) = state.toolbar.drag.take() {
                state.toolbar.pressed = None;
                if drag.active
                    && let Some(target) = layout.insertion(*position)
                {
                    let mut items = state.toolbar.profile.items(&state.toolbar.config).to_vec();
                    let item = items.remove(drag.source);
                    let target = target
                        .saturating_sub(usize::from(target > drag.source))
                        .min(items.len());
                    items.insert(target, item);
                    save(state, items, Some(target));
                }
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
                if let Some(Hit::Item(index)) = state.toolbar.pressed {
                    state.toolbar.selected = Some(index);
                    state.toolbar.recording = false;
                    state.toolbar.drag = Some(Drag {
                        source: index,
                        start: *position,
                        target: index,
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
            if input.key == Some(Key::Escape)
                && (state.toolbar.recording || state.toolbar.drag.is_some())
            {
                state.toolbar.cancel();
            } else if state.toolbar.recording {
                if let Some(key) = shortcut_key(state)
                    && let Some(chord) = shortcuts::chord(input)
                    && state.save_field("keys", key, chord.into())
                {
                    state.toolbar.recording = false;
                }
            } else {
                match input.key {
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
