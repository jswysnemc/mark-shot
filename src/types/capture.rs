pub type DamageRect = (u32, u32, u32, u32);
pub struct Placement {
    pub size: (i32, i32),
    pub position: (i32, i32),
    pub scale: f32,
}

impl Placement {
    /// Where this monitor starts in global coordinates, ready for drawing.
    pub fn offset(&self) -> (f32, f32) {
        (self.position.0 as f32, self.position.1 as f32)
    }

    pub fn rect(&self) -> Option<tiny_skia::Rect> {
        tiny_skia::Rect::from_xywh(0.0, 0.0, self.size.0 as f32, self.size.1 as f32)
    }

    pub fn px_size(&self) -> (u32, u32) {
        let (w, h) = self.size;
        px_size((w.max(1) as u32, h.max(1) as u32), self.scale)
    }
}

/// rounding for both the app's canvas and the compositor's buffer
pub fn px_size((w, h): (u32, u32), scale: f32) -> (u32, u32) {
    let px = |v: u32| ((v as f32 * scale).round() as u32).max(1);
    (px(w), px(h))
}
// Wayland outputs
use smithay_client_toolkit::output::OutputInfo as SctkOutputInfo;
use wayland_client::protocol::wl_output;

#[derive(Debug, Clone)]
pub struct Output {
    pub wl_output: wl_output::WlOutput,
    pub info: SctkOutputInfo,
}

impl Output {
    /// physical pixels per logical one
    /// (compoared by longer side to avoid issues with rotated)
    ///
    /// not using preferred scale because it is sent late
    pub fn scale(&self) -> f32 {
        let physical = self
            .info
            .modes
            .iter()
            .find(|m| m.current)
            .map(|m| m.dimensions.0.max(m.dimensions.1));
        let logical = self.info.logical_size.map(|(w, h)| w.max(h));
        let (Some(physical), Some(logical)) = (physical, logical) else {
            return 1.0;
        };
        if physical <= 0 || logical <= 0 {
            return 1.0;
        }
        let scale = (physical as f32 / logical as f32 * 120.0).round() / 120.0;
        // to be sure it won't take a lot of space
        scale.clamp(0.25, 8.0)
    }
}

// Pipewire and pixels
pub struct StreamInfo {
    pub node_id: u32,
    pub size: Option<(i32, i32)>,
    pub position: Option<(i32, i32)>,
}

pub struct MonitorFrame {
    pub output: usize,
    pub pixels: Vec<u8>,
    pub pw_width: u32,
    pub pw_height: u32,
    pub pw_stride: u32,
    pub info: StreamInfo,
}

pub struct Capture {
    pub pixmap: tiny_skia::Pixmap,
    pub scale: f32,
}

impl Capture {
    pub fn to_native(&self, point: (f64, f64)) -> (f64, f64) {
        let scale = self.scale as f64;
        (point.0 * scale, point.1 * scale)
    }
}

pub struct CaptureResult {
    pub frames: Vec<MonitorFrame>,
}
