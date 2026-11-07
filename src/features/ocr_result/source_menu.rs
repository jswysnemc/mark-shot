//! 【文字识别】【原文右键菜单】保留正文光标和选择，独立菜单像素与输入不混入表格格式菜单

use super::{source_menu_actions as actions, window::ResultWindow};
use crate::{
    backend::window::{WindowAction, WindowPopupAction},
    renderer::Painter,
    types::popup::{PopupEvent, PopupFrame},
    ui::context_menu::{ContextMenu, Event, Row},
};

#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Kind {
    #[default]
    Text,
    Image,
    Table,
    More,
    Translation,
    Language,
}

/// 【文字识别】【菜单打开】用 state 和 position 打开原文菜单，返回重绘；不移动光标，不修改选择或确认正文
pub(super) fn open(state: &mut ResultWindow, position: (f64, f64)) -> WindowAction {
    open_field(state, position, super::translation::input::Field::Source)
}

/// 【文字识别】【字段菜单】按 state、position 和 field 打开所属编辑器菜单，右键保持选择，原文、译文与语言使用不同身份
pub(super) fn open_field(
    state: &mut ResultWindow,
    position: (f64, f64),
    field: super::translation::input::Field,
) -> WindowAction {
    let clipboard = crate::utils::paste_from_clipboard().is_some_and(|text| !text.is_empty());
    let rows = if field == super::translation::input::Field::Source {
        actions::rows(state, clipboard)
    } else {
        actions::rows_for(state, field, clipboard)
    };
    if state.focus_target == super::control_input::FocusTarget::Language
        && field != super::translation::input::Field::Language
    {
        super::translation::controller::commit_target(state);
    }
    let (kind, focus) = match field {
        super::translation::input::Field::Source => {
            (Kind::Text, super::control_input::FocusTarget::Content)
        }
        super::translation::input::Field::Translation => (
            Kind::Translation,
            super::control_input::FocusTarget::Translation,
        ),
        super::translation::input::Field::Language => {
            (Kind::Language, super::control_input::FocusTarget::Language)
        }
        super::translation::input::Field::Table => return WindowAction::None,
    };
    state.focus_target = focus;
    if focus != super::control_input::FocusTarget::Language {
        state.text_focus = focus;
    }
    let action = open_rows(state, position, kind, rows);
    state.popup_editor_id = super::translation::input::editor(state, field)
        .map(|editor| (field, editor.input_identity()));
    action
}

/// 【文字识别】【共享原生菜单】按 state、position、kind 和 rows 打开独立菜单并取消交互事务，返回父窗口动作
pub(super) fn open_rows(
    state: &mut ResultWindow,
    position: (f64, f64),
    kind: Kind,
    rows: Vec<Row>,
) -> WindowAction {
    state.more_menu.clear();
    state.translation.language_menu = None;
    state.popup_editor_id = None;
    super::input_method::cancel_non_text(state);
    state.dragging = false;
    state.preview.release();
    state.last_click = None;
    state.control_press = None;
    let rows = rows
        .into_iter()
        .map(|mut row| {
            row.label = state.language.text(row.label);
            row
        })
        .collect();
    let mut menu = ContextMenu::new(
        rows,
        &mut state.fonts,
        (position.0 as f32, position.1 as f32),
        (state.layout.body.right(), state.layout.body.bottom()),
        0,
    );
    menu.id = crate::features::text::input_method::focus::identity();
    state.popup_kind = kind;
    state.source_menu = Some(menu);
    WindowAction::Redraw
}

/// 【文字识别】【菜单帧】按 state、scale 返回变化后的独立菜单像素，静止帧返回空值；不合成到父窗口
pub(super) fn frame(state: &mut ResultWindow, scale: f32) -> Option<PopupFrame<'_>> {
    let menu = state.source_menu.as_mut()?;
    frame_menu(menu, &mut state.fonts, &mut state.cache, scale)
}

/// 【文字识别】【菜单共用帧】按 menu、fonts、cache、scale 返回变化后的自有缓冲，不复制静止像素，不绘入父画布
pub(super) fn frame_menu<'a>(
    menu: &'a mut ContextMenu,
    fonts: &mut cosmic_text::FontSystem,
    cache: &mut cosmic_text::SwashCache,
    scale: f32,
) -> Option<PopupFrame<'a>> {
    let size = (
        (menu.layout.size.0 * scale).ceil() as u32,
        (menu.layout.size.1 * scale).ceil() as u32,
    );
    if menu.native
        && !menu.dirty
        && menu.drawn_theme == Some(crate::theme::Mode::current())
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
            .with_text(fonts, cache),
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
    if state.popup_kind == Kind::More {
        return super::more_menu::event(state, id, event);
    }
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
        PopupEvent::Scroll { .. } => Event::None,
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
    if state.more_menu.close_child(id) {
        return WindowAction::Redraw;
    }
    if !state.source_menu.as_ref().is_some_and(|menu| menu.id == id) {
        return WindowAction::None;
    }
    let menu = state.source_menu.take().unwrap();
    let owner = state.popup_editor_id.take();
    if owner.is_some_and(|(field, id)| {
        super::translation::input::editor(state, field)
            .is_none_or(|editor| editor.input_identity() != id)
    }) {
        return WindowAction::Redraw;
    }
    if state.popup_kind == Kind::More {
        state.more_menu.clear();
        return WindowAction::Redraw;
    }
    if let Some(index) = action.filter(|&index| {
        menu.rows
            .get(index)
            .is_some_and(|row| row.enabled && !row.separator())
    }) {
        match state.popup_kind {
            Kind::Text => actions::activate(state, index),
            Kind::Image => super::source_image_menu::activate(state, index),
            Kind::Table => super::table_menu::activate(state, index),
            Kind::More => WindowAction::Redraw,
            Kind::Translation => {
                actions::activate_for(state, super::translation::input::Field::Translation, index)
            }
            Kind::Language => {
                actions::activate_for(state, super::translation::input::Field::Language, index)
            }
        }
    } else {
        WindowAction::Redraw
    }
}
