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
    pub(crate) capture_logical_size: Option<(u32, u32)>,
    pub(crate) capture_buffer_size: Option<(u32, u32)>,
    pub(crate) fullscreen_output: Option<wayland_client::protocol::wl_output::WlOutput>,
    pub(crate) bootstrap_size: Option<(u32, u32)>,
    pub(crate) bootstrap_buffer: Option<ShmBuffer>,
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
        map_canvas_point(self.canvas_rect, (self.width, self.height), point)
    }

    /// 【截图覆盖层】【缓冲尺寸】返回当前表面应使用的共享像素网格，视口截图窗口固定保持捕获物理尺寸
    pub fn buffer_size(&self) -> (u32, u32) {
        resolve_buffer_size(
            self.capture_buffer_size,
            (self.width, self.height),
            self.scale,
        )
    }

    /// 【截图覆盖层】【坐标映射同步】当窗口实际配置尺寸偏离输出逻辑尺寸时启用线性指针映射，全屏一致时保持直通
    pub(crate) fn sync_capture_canvas_rect(&mut self) {
        if self.capture_logical_size.is_some() {
            self.canvas_rect =
                capture_canvas_rect(self.capture_logical_size, (self.width, self.height));
        }
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

/// 【截图覆盖层】【像素网格裁决】按固定捕获缓冲尺寸、窗口逻辑尺寸和比例返回实际共享内存缓冲尺寸
fn resolve_buffer_size(
    capture_buffer_size: Option<(u32, u32)>,
    configured: (u32, u32),
    scale: f32,
) -> (u32, u32) {
    capture_buffer_size.unwrap_or_else(|| crate::types::px_size(configured, scale))
}

/// 【截图覆盖层】【逻辑映射范围】按预期输出逻辑尺寸与合成器实际配置尺寸生成画布映射矩形，一致时返回空值
fn capture_canvas_rect(
    capture_logical_size: Option<(u32, u32)>,
    configured: (u32, u32),
) -> Option<tiny_skia::Rect> {
    let (logical_w, logical_h) = capture_logical_size?;
    if configured == (logical_w, logical_h) {
        None
    } else {
        tiny_skia::Rect::from_xywh(0.0, 0.0, logical_w.max(1) as f32, logical_h.max(1) as f32)
    }
}

/// 【截图覆盖层】【窗口坐标换算】按可选 canvas_rect 与窗口尺寸将窗口内指针坐标换算为逻辑画布坐标
fn map_canvas_point(
    canvas_rect: Option<tiny_skia::Rect>,
    window_size: (u32, u32),
    point: (f64, f64),
) -> (f64, f64) {
    canvas_rect.map_or(point, |rect| {
        (
            rect.left() as f64 + point.0 * rect.width() as f64 / window_size.0.max(1) as f64,
            rect.top() as f64 + point.1 * rect.height() as f64 / window_size.1.max(1) as f64,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【截图覆盖层测试】【平铺配置兼容】无 layer-shell 的窗口收到平铺尺寸时仍保持原生捕获缓冲并映射指针坐标
    #[test]
    fn tiled_capture_window_keeps_native_buffer_and_maps_pointer_until_fullscreen() {
        let capture_px = Some((2560, 1600));
        let capture_logical = Some((1280, 800));
        assert_eq!(
            resolve_buffer_size(capture_px, (635, 748), 2.0),
            (2560, 1600)
        );
        let rect = capture_canvas_rect(capture_logical, (635, 748));
        assert!(rect.is_some());
        let mapped = map_canvas_point(rect, (635, 748), (635.0, 748.0));
        assert!((mapped.0 - 1280.0).abs() < 1e-6);
        assert!((mapped.1 - 800.0).abs() < 1e-6);

        let fullscreen_rect = capture_canvas_rect(capture_logical, (1280, 800));
        assert!(fullscreen_rect.is_none());
        assert_eq!(
            map_canvas_point(fullscreen_rect, (1280, 800), (635.0, 748.0)),
            (635.0, 748.0)
        );
    }
}
