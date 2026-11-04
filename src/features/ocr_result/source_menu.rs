//! 【文字识别】【原文右键菜单】保留正文光标和选择，独立菜单像素与输入不混入表格格式菜单

use super::{source_menu_actions as actions, window::ResultWindow};
use crate::{
    backend::wayland::window::{WindowAction, WindowPopupAction},
    renderer::Painter,
    types::popup::{PopupEvent, PopupFrame},
    ui::context_menu::{ContextMenu, Event},
};

#[cfg(test)]
mod tests;

/// 【文字识别】【菜单打开】用 state 和 position 打开原文菜单，返回重绘；不移动光标，不修改选择或确认正文
pub(super) fn open(state: &mut ResultWindow, position: (f64, f64)) -> WindowAction {
    super::input_method::cancel(state);
    state.dragging = false;
    state.last_click = None;
    state.control_focus = None;
    state.menu = false;
    let clipboard = crate::utils::paste_from_clipboard().is_some_and(|text| !text.is_empty());
    let mut menu = ContextMenu::new(
        actions::rows(state, clipboard),
        &mut state.fonts,
        (position.0 as f32, position.1 as f32),
        (state.layout.body.right(), state.layout.body.bottom()),
        0,
    );
    menu.id = crate::features::text::input_method::focus::identity();
    state.source_menu = Some(menu);
    WindowAction::Redraw
}

/// 【文字识别】【菜单帧】按 state、scale 返回变化后的独立菜单像素，静止帧返回空值；不合成到父窗口
pub(super) fn frame(state: &mut ResultWindow, scale: f32) -> Option<PopupFrame<'_>> {
    let menu = state.source_menu.as_mut()?;
    let size = (
        (menu.layout.size.0 * scale).ceil() as u32,
        (menu.layout.size.1 * scale).ceil() as u32,
    );
    if menu.native
        && !menu.dirty
        && menu
            .pixmap
            .as_ref()
            .is_some_and(|pixels| (pixels.width(), pixels.height()) == size)
    {
        return None;
    }
    let mut context = tiny_skia::Pixmap::new(1, 1)?;
    crate::ui::context_menu::draw::prepare(
        &mut Painter::new(&mut context)
            .with_scale(scale)
            .with_text(&mut state.fonts, &mut state.cache),
        menu,
    );
    menu.native = true;
    let pixels = menu.pixmap.as_ref()?;
    Some(PopupFrame {
        id: menu.id,
        monitor_idx: 0,
        serial: None,
        anchor: menu.anchor,
        size: (menu.layout.size.0 as u32, menu.layout.size.1 as u32),
        scale,
        pixels: pixels.data(),
        pixel_size: (pixels.width(), pixels.height()),
    })
}

/// 【文字识别】【首字母导航】按 menu 当前高亮和 text 在启用行之间循环，返回状态事件；不执行动作或编辑正文
fn mnemonic(menu: &mut ContextMenu, text: &str) -> Event {
    let Some(letter) = text.chars().next().filter(|ch| ch.is_ascii_alphabetic()) else {
        return Event::None;
    };
    let matches: Vec<_> = menu
        .rows
        .iter()
        .enumerate()
        .filter(|(_, row)| {
            row.enabled
                && row
                    .label
                    .chars()
                    .next()
                    .is_some_and(|ch| ch.eq_ignore_ascii_case(&letter))
        })
        .map(|(index, _)| index)
        .collect();
    let next = matches
        .iter()
        .copied()
        .find(|&index| menu.active.is_none_or(|active| index > active))
        .or_else(|| matches.first().copied());
    if next.is_none() || next == menu.active {
        return Event::None;
    }
    menu.active = next;
    menu.dirty = true;
    Event::Changed
}

/// 【文字识别】【菜单事件】只处理 state 当前 id 的 event，返回菜单状态；快捷键不穿透正文，禁用项不交付动作
pub(super) fn event(state: &mut ResultWindow, id: u64, event: PopupEvent) -> WindowPopupAction {
    let Some(menu) = state.source_menu.as_mut().filter(|menu| menu.id == id) else {
        return WindowPopupAction::None;
    };
    let result = match event {
        PopupEvent::Configured { position } => {
            menu.position = (position.0 as f32, position.1 as f32);
            Event::None
        }
        PopupEvent::Pointer { x, y } => menu.pointer(
            0,
            (
                f64::from(menu.position.0) + x,
                f64::from(menu.position.1) + y,
            ),
        ),
        PopupEvent::Leave => menu.pointer(usize::MAX, (0.0, 0.0)),
        PopupEvent::Button {
            x,
            y,
            button,
            pressed,
        } => menu.button(
            0,
            (
                f64::from(menu.position.0) + x,
                f64::from(menu.position.1) + y,
            ),
            button,
            pressed,
        ),
        PopupEvent::Key { chord, text } => {
            let result = chord.map_or(Event::None, |chord| menu.key(chord.key, chord.mods.shift));
            if result == Event::None
                && chord.is_none_or(|chord| !chord.mods.ctrl && !chord.mods.alt && !chord.mods.logo)
            {
                text.map_or(result, |text| mnemonic(menu, &text))
            } else {
                result
            }
        }
        PopupEvent::Closed => Event::Close,
    };
    match result {
        Event::None => WindowPopupAction::None,
        Event::Changed => WindowPopupAction::Redraw,
        Event::Close => WindowPopupAction::Close,
        Event::Activate(index) => WindowPopupAction::Activate(index),
    }
}

/// 【文字识别】【菜单关闭】回收 state 当前 id 的菜单后执行 action，空值为取消；旧编号或禁用行不修改正文，返回窗口动作
pub(super) fn closed(state: &mut ResultWindow, id: u64, action: Option<usize>) -> WindowAction {
    if !state.source_menu.as_ref().is_some_and(|menu| menu.id == id) {
        return WindowAction::None;
    }
    let menu = state.source_menu.take().unwrap();
    if let Some(index) = action.filter(|&index| {
        menu.rows
            .get(index)
            .is_some_and(|row| row.enabled && !row.separator())
    }) {
        actions::activate(state, index)
    } else {
        WindowAction::Redraw
    }
}
