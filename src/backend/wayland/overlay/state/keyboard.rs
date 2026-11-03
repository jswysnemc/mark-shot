// ── keyboard input handling ──────────────────────────────────────────────────
// handlers shortcuts using a two-tier approach:
// 1. first, checks 'Keysym' to match shortcuts by their actual letter/symbol
// 2. if not latin symbol matches (e.g, user is on a cyrillic layout), it falls
//    back to 'raw_code' to trigger the shortcut based on the physical key position

use smithay_client_toolkit::seat::keyboard::{
    KeyEvent, KeyboardHandler, Keysym, Modifiers, RepeatInfo,
};
use wayland_client::protocol::{wl_keyboard, wl_surface};
use wayland_client::{Connection, QueueHandle};

use crate::backend::wayland::keysym;
use crate::backend::wayland::overlay::state::OverlayState;
use crate::keys::Key;
use crate::types::OverlayEvent;
impl KeyboardHandler for OverlayState {
    fn press_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        self.process_key(&event);
    }
    fn repeat_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        mut event: KeyEvent,
    ) {
        if event.utf8.is_none() {
            event.utf8 = event.keysym.key_char().map(String::from);
        }
        self.process_key(&event);
    }

    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        modifiers: Modifiers,
        _: smithay_client_toolkit::seat::keyboard::RawModifiers,
        _: u32,
    ) {
        self.mods = keysym::mods(&modifiers);
        self.events.push_back(OverlayEvent::ModifiersChanged {
            ctrl: self.mods.ctrl,
            shift: self.mods.shift,
        });
    }

    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        surface: &wl_surface::WlSurface,
        _: u32,
        _: &[u32],
        _: &[Keysym],
    ) {
        if let Some(popup) = self.popup.as_mut()
            && popup.popup.wl_surface() == surface
        {
            popup.keyboard_focused = true;
            return;
        }
        if let Some(monitor_idx) = self
            .surfaces
            .iter()
            .find(|(_, sd)| &sd.surface == surface)
            .map(|(id, _)| *id)
        {
            self.events.push_back(OverlayEvent::Focus { monitor_idx });
        }
    }
    fn release_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        _event: KeyEvent,
    ) {
    }
    fn update_repeat_info(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _info: RepeatInfo,
    ) {
    }
    /// 【Wayland输入】【焦点离开】按 surface 交付主表面失焦或当前菜单失焦；不提交文字，其他参数来自协议，无返回值
    fn leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        surface: &wl_surface::WlSurface,
        _: u32,
    ) {
        if let Some(monitor_idx) = self
            .surfaces
            .iter()
            .find(|(_, data)| &data.surface == surface)
            .map(|(index, _)| *index)
        {
            self.events
                .push_back(OverlayEvent::FocusLost { monitor_idx });
        }
        if self
            .popup
            .as_ref()
            .is_some_and(|popup| popup.keyboard_focused && popup.popup.wl_surface() == surface)
        {
            self.dismiss_popup();
        }
    }
}

impl OverlayState {
    fn process_key(&mut self, event: &KeyEvent) {
        let chord = keysym::chord(event.keysym, event.raw_code, self.mods);
        let editing_key = chord.is_some_and(|c| {
            matches!(
                c.key,
                Key::Escape
                    | Key::Enter
                    | Key::Backspace
                    | Key::Delete
                    | Key::Left
                    | Key::Right
                    | Key::Up
                    | Key::Down
                    | Key::Home
                    | Key::End
            )
        });
        let text = event
            .utf8
            .clone()
            .filter(|t| !t.is_empty() && !self.mods.ctrl && !editing_key);
        if chord.is_some() || text.is_some() {
            if self.popup.is_some() {
                self.popup_event(crate::types::popup::PopupEvent::Key { chord, text });
            } else {
                self.events.push_back(OverlayEvent::Key { chord, text });
            }
        }
    }
}
