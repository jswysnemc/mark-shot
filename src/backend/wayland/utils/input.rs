// ── input region helpers ─────────────────────────────────────────────────────
// Input regions may only be updated while the surface is mapped; committing
// before or after unmap causes the compositor to drop connection on invalid sizes.

use smithay_client_toolkit::compositor::{CompositorState, Region};
use wayland_client::protocol::wl_surface;

/// Restricts pointer input on `surface` to `rect`, or allows all events through when `None`.
pub fn apply_input_region(
    compositor: &CompositorState,
    surface: &wl_surface::WlSurface,
    rect: Option<(i32, i32, i32, i32)>,
) {
    let Ok(region) = Region::new(compositor) else {
        return;
    };
    if let Some((x, y, w, h)) = rect {
        region.add(x, y, w, h);
    }
    surface.set_input_region(Some(region.wl_region()));
}
