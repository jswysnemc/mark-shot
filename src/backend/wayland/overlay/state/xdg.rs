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
            if let Some(surface) = self.surfaces.values().find(|surface| {
                &surface.surface == window.wl_surface() && surface.shm_buffer.is_some()
            }) {
                // 1. 【滚动截图】【父窗口配置】缓冲已由 configure_surface 按尺寸初始化并附着，此处只提交窗口几何状态
                window.set_window_geometry(0, 0, surface.width, surface.height);
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
        if self.image_window.is_none()
            && let Some(surface) = self
                .surfaces
                .values_mut()
                .find(|surface| &surface.surface == window.wl_surface())
            && surface.fullscreen_output.is_some()
        {
            let expected = surface.capture_logical_size;
            let is_fullscreen = configure.is_fullscreen()
                || size.is_none_or(|configured| Some(configured) == expected);
            if !is_fullscreen && surface.bootstrap_size.is_none() && surface.shm_buffer.is_none() {
                // 1. 【截图覆盖层】【全屏引导】平铺合成器首轮配置未进入全屏状态时，附着透明缓冲完成映射并重申全屏请求
                let fallback = size.unwrap_or((surface.width.max(1), surface.height.max(1)));
                let (bw, bh) = if surface.viewport.is_some() {
                    (1, 1)
                } else {
                    fallback
                };
                match crate::backend::wayland::utils::shm::create_shm_buffer(&mut self.pool, bw, bh)
                {
                    Ok(mut buffer) => {
                        buffer.clear(&mut self.pool);
                        if let Some(viewport) = &surface.viewport {
                            viewport.set_destination(fallback.0 as i32, fallback.1 as i32);
                        }
                        window.set_window_geometry(0, 0, fallback.0, fallback.1);
                        if let Some(output) = &surface.fullscreen_output {
                            window.set_fullscreen(Some(output));
                        }
                        if let Err(error) = buffer.buffer.attach_to(&surface.surface) {
                            self.configure_error = Some(error.to_string());
                            return;
                        }
                        surface.surface.damage_buffer(0, 0, bw as i32, bh as i32);
                        surface.surface.commit();
                        surface.bootstrap_size = Some(fallback);
                        surface.bootstrap_buffer = Some(buffer);
                        return;
                    }
                    Err(error) => {
                        self.configure_error = Some(error.to_string());
                        return;
                    }
                }
            }
        }
        self.configure_surface(window.wl_surface(), size);
        if self.image_window.is_none()
            && let Some(surface) = self
                .surfaces
                .values()
                .find(|surface| &surface.surface == window.wl_surface())
        {
            window.set_window_geometry(0, 0, surface.width.max(1), surface.height.max(1));
        }
        if self.image_window.is_some()
            && let Some(surface) = self
                .surfaces
                .values()
                .find(|surface| &surface.surface == window.wl_surface())
        {
            let actual = (surface.width, surface.height);
            self.image_maximized = configure.is_maximized();
            self.image_window = Some(actual);
            window.set_window_geometry(0, 0, actual.0, actual.1);
            self.events
                .retain(|event| !matches!(event, OverlayEvent::CanvasResized { .. }));
            self.events
                .push_back(OverlayEvent::CanvasResized { size: actual });
        }
    }

    fn request_close(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &Window) {
        self.events.push_back(OverlayEvent::EscapePressed);
    }
}
