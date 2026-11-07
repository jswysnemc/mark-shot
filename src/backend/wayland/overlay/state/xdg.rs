// ── xdg ───────────────────────────────────────────────────────────────────────────────────────
// - xdg_shell: used for managing desktop windows (doesn't work on gnome)

use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::xdg::XdgSurface;
use smithay_client_toolkit::shell::xdg::window::{Window, WindowConfigure, WindowHandler};

use wayland_client::{Connection, QueueHandle};

use crate::backend::wayland::overlay::state::OverlayState;
use crate::types::OverlayEvent;

impl WindowHandler for OverlayState {
    /// 【截图覆盖层】【窗口配置】按 window 与 configure 更新尺寸，控制父缓冲保持透明；连接、队列及序列号不参与坐标换算，无返回值
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
        if self.surfaces.values().any(|surface| {
            matches!(&surface.role, crate::backend::wayland::utils::surface::Role::ScrollParent(parent) if parent.wl_surface() == window.wl_surface())
        }) {
            self.configure_surface(window.wl_surface(), size);
            if let Some(surface) = self.surfaces.values_mut().find(|surface| &surface.surface == window.wl_surface())
                && let Some(buffer) = &mut surface.shm_buffer {
                    buffer.clear(&mut self.pool);
                    window.set_window_geometry(0, 0, surface.width, surface.height);
                    surface.surface.attach(Some(buffer.wl_buffer()), 0, 0);
                    surface.surface.commit();
            }
            return;
        }
        for surface in self.surfaces.values_mut() {
            if let crate::backend::wayland::utils::surface::Role::ScrollPopup(popup) =
                &mut surface.role
                && popup.parent.wl_surface() == window.wl_surface()
            {
                if let Err(error) =
                    popup.map_parent(size.unwrap_or(popup.parent_size), &mut self.pool)
                {
                    self.configure_error = Some(error.to_string());
                }
                return;
            }
        }
        self.configure_surface(window.wl_surface(), size);
    }

    fn request_close(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &Window) {
        self.events.push_back(OverlayEvent::EscapePressed);
    }
}
