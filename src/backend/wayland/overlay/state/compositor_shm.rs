// ── compositor and shm handlers ────────────────────────────────────
// just a reminder, its for graphical initialization:
// - wl_compositor: creating surfaces
// - wl_shm: deviding memory for transferring frame pixels

use smithay_client_toolkit::compositor::CompositorHandler;

use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shm::ShmHandler;

use wayland_client::protocol::{wl_output, wl_surface};
use wayland_client::{Connection, QueueHandle};

use crate::backend::wayland::overlay::state::OverlayState;
use crate::backend::wayland::utils::shm::create_shm_buffer;
use crate::types::OverlayEvent;
// ── compositor ────────────────────────────────────────────────────────────────────────────────
impl CompositorHandler for OverlayState {
    fn scale_factor_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: i32,
    ) {
    }
    fn transform_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: wl_output::Transform,
    ) {
    }
    fn frame(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        if self.popup_frame(surface, qh) {
            return;
        }
        if let Some((&monitor_idx, sd)) = self
            .surfaces
            .iter_mut()
            .find(|(_, sd)| &sd.surface == surface)
        {
            sd.frame_sent = None;
            if super::super::frame_staging::has_writable_buffer(sd) {
                sd.waiting_release = false;
            }
            self.events
                .push_back(OverlayEvent::FrameDone { monitor_idx });
        }
    }
    fn surface_enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        surface: &wl_surface::WlSurface,
        output: &wl_output::WlOutput,
    ) {
        if let Some(probe) = self.probe.as_mut()
            && probe.layer.wl_surface() == surface
        {
            probe.output = Some(output.clone());
        }
    }
    fn surface_leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
}
// ── shm ──────────────────────────────────────────────────────────────────────────────────────
impl ShmHandler for OverlayState {
    fn shm_state(&mut self) -> &mut smithay_client_toolkit::shm::Shm {
        &mut self.shm
    }
}

// with both the window and the layer surface we cannot work exclusively from the Wayland client side
// here we must wait for a response from the compositor itself. Also, we must use its width and height
// otherwise the compositor might reject the window. Only after obtaining this data do we create buffers
// and attach the window to the surface.

impl OverlayState {
    /// 【截图覆盖层】【缓冲配置】按surface和size映射输出，滚动父表面在具备视区协议时只使用透明像素，返回值为空
    pub fn configure_surface(&mut self, surface: &wl_surface::WlSurface, size: Option<(u32, u32)>) {
        let Some(sd) = self.surfaces.values_mut().find(|sd| &sd.surface == surface) else {
            return;
        };

        // to avoid livelock, if compositor sends configure without specifying sizes
        // we just use ours
        let (w, h) = size.unwrap_or((sd.width, sd.height));

        // if its already configured, and sizes weren't changed
        // there is no point of continuing
        if sd.shm_buffer.is_some() && sd.width == w && sd.height == h {
            return;
        }
        // creating buffers and attaching
        let pool = &mut self.pool;
        let transparent_parent = sd.scroll_layer.is_some()
            || matches!(
                &sd.role,
                crate::backend::wayland::utils::surface::Role::ScrollParent(_)
            );
        let (bw, bh) = if transparent_parent && sd.viewport.is_some() {
            (1, 1)
        } else {
            crate::types::px_size((w, h), sd.scale)
        };

        let mut shm_buffer = match create_shm_buffer(pool, bw, bh) {
            Ok(buffer) => buffer,
            Err(e) => {
                self.configure_error = Some(e.to_string());
                return;
            }
        };
        if transparent_parent {
            shm_buffer.clear(pool);
        }

        // apply configured input region with the first frame after mapping
        sd.mapped = true;
        sd.apply_input(&self.compositor_state);
        // viewport maps buffer, which is is in physical pixels, back to the logical size
        if let Some(viewport) = &sd.viewport {
            viewport.set_destination(w as i32, h as i32);
        }
        if transparent_parent {
            shm_buffer
                .buffer
                .attach_to(&sd.surface)
                .expect("newly created buffer is inactive");
        } else {
            shm_buffer
                .buffer
                .attach_to(&sd.surface)
                .expect("newly created buffer is inactive");
            sd.waiting_release = true;
        }
        sd.surface.damage_buffer(0, 0, bw as i32, bh as i32);
        sd.surface.commit();

        sd.shm_buffer = Some(shm_buffer);
        sd.spare_buffer = None;
        sd.width = w;
        sd.height = h;
    }
}
