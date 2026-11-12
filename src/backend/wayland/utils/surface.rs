// ── Wayland surface data container ───────────────────────────────────────────
// utility structure that bundles a raw Wayland surface, its layer_shell
// integration, and its allocated pixel buffers into a single logical window
//
// in 'utils' because it represents a pure data layout used by the
// sub-orchestrator ('overlay.rs'), keeping protocol handlers free of state management
// reminder: each output has its own surface

use crate::backend::wayland::utils::input;
use crate::backend::wayland::utils::shm::ShmBuffer;
use smithay_client_toolkit::compositor::CompositorState;
use std::time::Instant;
use wayland_client::protocol::{wl_subsurface, wl_surface};
use wayland_protocols::wp::viewporter::client::wp_viewport;

/// screenshot under overlay is with Native resolution, for it to be Rendered 1:1 while the overlay
/// only handles background dimming and UI drawing.
/// otherwise screenshot will be low quality when changed scales
pub struct Background {
    pub subsurface: wl_subsurface::WlSubsurface,
    pub surface: wl_surface::WlSurface,
    pub viewport: Option<wp_viewport::WpViewport>,
    pub buffer: ShmBuffer,
}

impl Drop for Background {
    fn drop(&mut self) {
        if let Some(viewport) = &self.viewport {
            viewport.destroy();
        }
        self.subsurface.destroy();
        self.surface.destroy();
    }
}

// NOT USED ANYWHERE
// for keeping role alive
pub enum Role {
    Window(smithay_client_toolkit::shell::xdg::window::Window),
    ScrollParent(smithay_client_toolkit::shell::xdg::window::Window),
    Layer(smithay_client_toolkit::shell::wlr_layer::LayerSurface),
    ScrollPopup(Box<super::scroll_popup::ScrollPopup>),
}

pub struct ScrollFrame {
    pub rgba: Vec<u8>,
    pub size: (u32, u32),
    pub stamp: Option<super::scroll_stamp::Stamp>,
}

pub struct PresentedPanel {
    pub stamp: super::scroll_stamp::Stamp,
    pub source: (u32, u32),
    pub logical: (u32, u32),
}

/// Scope of pointer events accepted by the overlay surface.
/// Kept on the surface state rather than immediately applied because unmapped
/// layer surfaces reject state commits with invalid_size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputRegion {
    #[default]
    Full,
    Rect(i32, i32, i32, i32),
    Nothing,
}

pub struct SurfaceData {
    pub background: Option<Background>,
    pub(crate) scroll_layer: Option<crate::backend::wayland::overlay::scroll_layer::ScrollLayer>,
    pub role: Role,
    pub surface: wl_surface::WlSurface,
    pub viewport: Option<wp_viewport::WpViewport>,

    pub shm_buffer: Option<ShmBuffer>,
    pub(crate) spare_buffer: Option<ShmBuffer>,
    pub(crate) buffer_history: crate::backend::wayland::overlay::buffer_history::History,
    pub(crate) waiting_release: bool,
    /// Whether the surface currently has a buffer attached.
    pub mapped: bool,
    /// Pending input region applied upon configure/commit.
    pub input: InputRegion,
    pub canvas_rect: Option<tiny_skia::Rect>,
    pub scroll_frame: Option<ScrollFrame>,
    pub scroll_stamps: std::collections::VecDeque<PresentedPanel>,

    pub width: u32,
    pub height: u32,
    pub scale: f32,
    pub frame_sent: Option<Instant>,
}

impl SurfaceData {
    /// 【滚动截图】【窗口坐标】将 point 从实际窗口映射到原面板画布，返回逻辑坐标；普通截图表面保持原坐标
    pub fn canvas_point(&self, point: (f64, f64)) -> (f64, f64) {
        self.canvas_rect.map_or(point, |rect| {
            (
                rect.left() as f64 + point.0 * rect.width() as f64 / self.width.max(1) as f64,
                rect.top() as f64 + point.1 * rect.height() as f64 / self.height.max(1) as f64,
            )
        })
    }
    pub fn buffer_size(&self) -> (u32, u32) {
        crate::types::px_size((self.width, self.height), self.scale)
    }

    /// Sets the pending input region on the surface without committing.
    pub fn apply_input(&self, compositor: &CompositorState) {
        match self.input {
            InputRegion::Full => self.surface.set_input_region(None),
            InputRegion::Rect(x, y, w, h) => {
                input::apply_input_region(compositor, &self.surface, Some((x, y, w, h)))
            }
            InputRegion::Nothing => input::apply_input_region(compositor, &self.surface, None),
        }
    }
}
