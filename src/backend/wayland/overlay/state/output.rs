// ── Output Management ────────────────────────────────────────────────────────
// A module for tracking connected outputs
//
// maintain an up to date list of 'Vec<Output>' inside 'OverlayState' to
// map them against our 'Placements'.
// rebuild_outputs method is called whenever any display configuration changes

use smithay_client_toolkit::output::{OutputHandler, OutputState};
use wayland_client::protocol::wl_output;
use wayland_client::{Connection, QueueHandle};

use crate::backend::wayland::overlay::state::OverlayState;
use crate::types::Output;

impl OutputHandler for OverlayState {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {
        self.rebuild_outputs();
    }
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {
        self.rebuild_outputs();
    }
    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {
        self.rebuild_outputs();
    }
}

impl OverlayState {
    /// Synchronizes our local 'outputs' list with the SCTK internal state
    /// This is called whenever the compositor notifies us about ANY display changes
    pub fn rebuild_outputs(&mut self) {
        self.outputs = self
            .output_state
            .outputs()
            .filter_map(|wl_output| {
                self.output_state
                    .info(&wl_output)
                    .map(|info| Output { wl_output, info })
            })
            .collect();
    }
}
