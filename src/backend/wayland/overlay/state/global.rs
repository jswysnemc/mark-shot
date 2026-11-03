// ── Custom Wayland Dispatchers ─────────────────────────────────────────────────────
// these handlers allow us to use wayland protocols that SCTK doesn't provide
// we manually process events for things like fractional scaling and surface viewports

use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::registry::ProvidesRegistryState;
use smithay_client_toolkit::{delegate_dispatch2, delegate_registry, registry_handlers};
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::wp::{
    fractional_scale::v1::client::{wp_fractional_scale_manager_v1, wp_fractional_scale_v1},
    viewporter::client::{wp_viewport, wp_viewporter},
};

use crate::backend::wayland::overlay::state::OverlayState;

// ── registry handling ─────────────────────────────────────────────────────────────
// reminder: this tells sctk: "I'm willing to manage standard Wayland global
// objects on my own using your registry tools"

impl ProvidesRegistryState for OverlayState {
    fn registry(&mut self) -> &mut smithay_client_toolkit::registry::RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState];
}

delegate_registry!(OverlayState);
delegate_dispatch2!(OverlayState);

// ── fractional scale protocol ──────────────────────────────────────────────────────
// handles HiDPI support. The server tells us the monitor's scale factor,
// and update our internal state scale accordingly
impl Dispatch<wp_fractional_scale_v1::WpFractionalScaleV1, ()> for OverlayState {
    fn event(
        state: &mut Self,
        _: &wp_fractional_scale_v1::WpFractionalScaleV1,
        event: wp_fractional_scale_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // wayland sends the scale factor as a fixed point integer (multiplied by 120)
        // we divide by 120.0 to convert it into a usual floating point scale (1.0, 1.25, 2.0, etc)
        if let wp_fractional_scale_v1::Event::PreferredScale { scale } = event {
            state.scale = scale as f64 / 120.0;
        }
    }
}

// ── viewporter & manager ───────────────────────────────────────────────────────────
// protocols are for scaling/cropping surfaces. Only use them to
// create objects (requests), so we don't need to handle any server events.
impl Dispatch<wp_viewporter::WpViewporter, ()> for OverlayState {
    fn event(
        _: &mut Self,
        _: &wp_viewporter::WpViewporter,
        _: wp_viewporter::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wp_viewport::WpViewport, ()> for OverlayState {
    fn event(
        _: &mut Self,
        _: &wp_viewport::WpViewport,
        _: wp_viewport::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1, ()> for OverlayState {
    fn event(
        _: &mut Self,
        _: &wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1,
        _: wp_fractional_scale_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
