//! 【设置下拉】【输入路由】浮层消费指针和键盘事件，取消操作不写入配置

use super::super::{
    menus,
    window::{Preferences, contains},
};
use super::{
    Dropdown,
    layout::{Hit, Layout},
};
use crate::{
    backend::window::{LEFT_BUTTON, WindowAction, WindowEvent},
    keys::Key,
    ui::{fonts::UiFonts, scrollbar},
};

enum Outcome {
    Keep,
    Close,
    Choose(usize),
}

/// 【设置下拉】【窗口事件】按 event 更新 state 浮层，返回消费结果；异步完成通知继续交给业务页面
pub(in crate::features::preferences) fn event(
    state: &mut Preferences,
    event: &WindowEvent,
) -> Option<WindowAction> {
    if matches!(event, WindowEvent::Wake | WindowEvent::Resize { .. }) {
        if matches!(event, WindowEvent::Resize { .. }) {
            state.dropdown = None;
        }
        return None;
    }
    let mut menu = state.dropdown.take()?;
    if let WindowEvent::PointerMove { position } | WindowEvent::PointerButton { position, .. } =
        event
    {
        state.pointer = *position;
    }
    let outcome = handle(&mut menu, event, state.size, &mut state.fonts);
    match outcome {
        Outcome::Close => {
            state.pressed = None;
        }
        Outcome::Keep => state.dropdown = Some(menu),
        Outcome::Choose(index) => {
            if !menus::choose(state, menu.target, &menu.options[index].value) {
                state.dropdown = Some(menu);
            }
        }
    }
    Some(WindowAction::Redraw)
}

/// 【设置下拉】【交互状态】按 event、size 和 fonts 更新 menu，返回保留、取消或确认选项
fn handle(
    menu: &mut Dropdown,
    event: &WindowEvent,
    size: (u32, u32),
    fonts: &mut UiFonts,
) -> Outcome {
    let layout = Layout::new(menu, size);
    match event {
        WindowEvent::Focused(false) => return Outcome::Close,
        WindowEvent::PointerMove { position } => {
            if let Some(drag) = menu.drag
                && let Some(bar) = &layout.scrollbar
            {
                menu.offset = drag.value(*position, bar);
            } else if menu.pressed == Some(Hit::Search)
                && let Some(bounds) = layout.search
                && let Some(search) = &mut menu.search
            {
                search.pointer(
                    fonts,
                    (
                        position.0 - f64::from(bounds.left() + 29.0),
                        position.1 - f64::from(bounds.top() + 3.0),
                    ),
                    true,
                );
            }
        }
        WindowEvent::PointerButton {
            position,
            button: LEFT_BUTTON,
            pressed,
        } => {
            if *pressed {
                if !contains(layout.bounds, *position) {
                    return Outcome::Close;
                }
                menu.pressed = layout.hit(*position);
                if menu.pressed == Some(Hit::Scrollbar)
                    && let Some(bar) = &layout.scrollbar
                {
                    if contains(bar.handle, *position) {
                        menu.drag = Some(scrollbar::Drag::new(*position, bar));
                    } else if position.1 < f64::from(bar.handle.top()) {
                        menu.offset = menu.offset.saturating_sub(bar.page);
                    } else {
                        menu.offset = (menu.offset + bar.page).min(bar.maximum);
                    }
                } else if menu.pressed == Some(Hit::Search)
                    && let Some(bounds) = layout.search
                    && let Some(search) = &mut menu.search
                {
                    search.pointer(
                        fonts,
                        (
                            position.0 - f64::from(bounds.left() + 29.0),
                            position.1 - f64::from(bounds.top() + 3.0),
                        ),
                        false,
                    );
                }
            } else {
                menu.drag = None;
                if let Some(Hit::Option(index)) = menu
                    .pressed
                    .take()
                    .filter(|hit| Some(*hit) == layout.hit(*position))
                {
                    return Outcome::Choose(index);
                }
            }
        }
        WindowEvent::Scroll { position, delta } => {
            if !contains(layout.bounds, *position) {
                return Outcome::Close;
            }
            let step = if delta.1 > 0.0 {
                1
            } else if delta.1 < 0.0 {
                -1
            } else {
                0
            };
            menu.offset = menu
                .offset
                .saturating_add_signed(step)
                .min(menu.filtered.len().saturating_sub(layout.capacity));
        }
        WindowEvent::Key(input) => match input.key {
            Some(Key::Escape | Key::Tab) => return Outcome::Close,
            Some(Key::Enter) => {
                if let Some(index) = menu.cursor {
                    return Outcome::Choose(index);
                }
            }
            Some(Key::Space) if menu.search.is_none() => {
                if let Some(index) = menu.cursor {
                    return Outcome::Choose(index);
                }
            }
            Some(Key::Up | Key::Down | Key::PageUp | Key::PageDown | Key::Home | Key::End)
                if !input.mods.ctrl =>
            {
                let current = menu
                    .cursor
                    .and_then(|cursor| menu.filtered.iter().position(|index| *index == cursor))
                    .unwrap_or(0);
                let next = match input.key {
                    Some(Key::Up) => current.saturating_sub(1),
                    Some(Key::Down) => current + 1,
                    Some(Key::PageUp) => current.saturating_sub(layout.capacity),
                    Some(Key::PageDown) => current + layout.capacity,
                    Some(Key::Home) => 0,
                    _ => menu.filtered.len().saturating_sub(1),
                }
                .min(menu.filtered.len().saturating_sub(1));
                menu.cursor = menu.filtered.get(next).copied();
                menu.reveal(layout.capacity);
            }
            _ => {
                if let Some(search) = &mut menu.search {
                    let before = search.text();
                    search.key(fonts, input);
                    if search.text() != before {
                        menu.filter();
                        menu.reveal(Layout::new(menu, size).capacity);
                    }
                }
            }
        },
        _ => {}
    }
    Outcome::Keep
}
