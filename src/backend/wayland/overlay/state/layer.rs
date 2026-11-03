use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::{
    LayerShellHandler, LayerSurface, LayerSurfaceConfigure,
};

use wayland_client::{Connection, QueueHandle};

use crate::backend::wayland::overlay::state::OverlayState;
use crate::backend::wayland::utils::shm::create_shm_buffer;
use crate::types::OverlayEvent;

impl LayerShellHandler for OverlayState {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, layer: &LayerSurface) {
        if self
            .surfaces
            .values()
            .any(|sd| &sd.surface == layer.wl_surface())
        {
            self.events.push_back(OverlayEvent::EscapePressed);
        }
    }

    /// besides usual configuration, we create a small 1x1 transparent buffer for the probe surface to find the focused monitor
    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _: u32,
    ) {
        let Some(probe) = self.probe.as_mut().filter(|probe| &probe.layer == layer) else {
            let (w, h) = configure.new_size;
            self.configure_surface(layer.wl_surface(), (w > 0 && h > 0).then_some((w, h)));
            return;
        };
        if probe.buffer.is_some() {
            return;
        }
        let Ok(mut buffer) = create_shm_buffer(&mut self.pool, 1, 1) else {
            return;
        };
        buffer.write_pixels(&mut self.pool, &[0; 4]);
        layer.wl_surface().attach(Some(buffer.wl_buffer()), 0, 0);
        layer.wl_surface().damage_buffer(0, 0, 1, 1);
        layer.commit();
        probe.buffer = Some(buffer);
    }
}
