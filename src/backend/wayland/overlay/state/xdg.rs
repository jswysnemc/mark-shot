// ── xdg ───────────────────────────────────────────────────────────────────────────────────────
// - xdg_shell: used for managing desktop windows (doesn't work on gnome)

use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::xdg::window::{Window, WindowConfigure, WindowHandler};

use wayland_client::{Connection, QueueHandle};

use crate::backend::wayland::overlay::state::OverlayState;
use crate::types::OverlayEvent;

impl WindowHandler for OverlayState {
    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        window: &Window,
        configure: WindowConfigure,
        _serial: u32,
    ) {
        let size = match configure.new_size {
            (Some(width), Some(height)) => Some((width.get(), height.get())),
            _ => None,
        };
        self.configure_surface(window.wl_surface(), size);
    }

    fn request_close(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &Window) {
        self.events.push_back(OverlayEvent::EscapePressed);
    }
}
