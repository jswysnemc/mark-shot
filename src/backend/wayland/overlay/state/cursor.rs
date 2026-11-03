// ── Cursor shape (wp_cursor_shape_device_v1) ────────────────────────────────
// using cursor-shape-v1

use crate::backend::wayland::overlay::state::OverlayState;
use crate::types::CursorIcon;
use wayland_protocols::wp::cursor_shape::v1::client::wp_cursor_shape_device_v1::Shape;

fn map_cursor_icon(icon: CursorIcon) -> Shape {
    match icon {
        CursorIcon::Default => Shape::Default,
        CursorIcon::Crosshair => Shape::Crosshair,
        CursorIcon::Pointer => Shape::Pointer,
        CursorIcon::Text => Shape::Text,
        CursorIcon::Grab => Shape::Grab,
        CursorIcon::Grabbing => Shape::Grabbing,
        CursorIcon::NsResize => Shape::NsResize,
        CursorIcon::EwResize => Shape::EwResize,
        CursorIcon::NeswResize => Shape::NeswResize,
        CursorIcon::NwseResize => Shape::NwseResize,
    }
}

impl OverlayState {
    pub fn apply_cursor(&mut self, icon: CursorIcon) {
        if icon == self.current_cursor_icon && self.cursor_applied_since_enter {
            return;
        }
        self.current_cursor_icon = icon;
        if let Some(device) = &self.cursor_shape_device {
            device.set_shape(self.pointer_enter_serial, map_cursor_icon(icon));
            self.cursor_applied_since_enter = true;
            self.pending_flush = true;
        }
    }
}
