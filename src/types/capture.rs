pub type DamageRect = (u32, u32, u32, u32);
#[derive(Clone)]
pub struct Placement {
    pub size: (i32, i32),
    pub position: (i32, i32),
    pub scale: f32,
    /// 捕获或原生窗口提供的真实像素尺寸，避免分数缩放取整后增加或丢失边缘像素
    pub native_size: Option<(u32, u32)>,
}

impl Placement {
    /// Where this monitor starts in global coordinates, ready for drawing.
    pub fn offset(&self) -> (f32, f32) {
        (self.position.0 as f32, self.position.1 as f32)
    }

    /// 【截图布局】【逻辑边界】无参数，返回真实像素对应的本地逻辑范围，保留非整除缩放的边缘
    pub fn rect(&self) -> Option<tiny_skia::Rect> {
        let (width, height) = self.native_size.map_or(
            (self.size.0 as f32, self.size.1 as f32),
            |(width, height)| (width as f32 / self.scale, height as f32 / self.scale),
        );
        tiny_skia::Rect::from_xywh(0.0, 0.0, width, height)
    }

    /// 【截图布局】【物理尺寸】无参数，返回真实像素网格；没有原生尺寸时按逻辑尺寸和比例换算
    pub fn px_size(&self) -> (u32, u32) {
        if let Some(size) = self.native_size {
            return size;
        }
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
#[cfg(target_os = "linux")]
use super::linux_output::Info as SctkOutputInfo;
#[cfg(target_os = "linux")]
use wayland_client::protocol::wl_output;

#[cfg(target_os = "linux")]
#[derive(Debug, Clone)]
pub struct Output {
    pub wl_output: Option<wl_output::WlOutput>,
    pub info: SctkOutputInfo,
}

#[cfg(target_os = "linux")]
impl Output {
    /// 【Linux输出】【逻辑尺寸】优先返回 xdg-output 逻辑尺寸，沙箱缺失扩展时按当前物理模式、旋转方向与整数缩放换算
    pub fn logical_size(&self) -> Option<(i32, i32)> {
        if let Some(logical) = self.info.logical_size {
            return Some(logical);
        }
        let mode = self.info.modes.iter().find(|mode| mode.current)?;
        let (width, height) = if matches!(
            self.info.transform,
            wl_output::Transform::_90
                | wl_output::Transform::_270
                | wl_output::Transform::Flipped90
                | wl_output::Transform::Flipped270
        ) {
            (mode.dimensions.1, mode.dimensions.0)
        } else {
            mode.dimensions
        };
        if width <= 0 || height <= 0 {
            return None;
        }
        let factor = self.info.scale_factor.max(1);
        Some((
            (width + factor - 1) / factor,
            (height + factor - 1) / factor,
        ))
    }

    /// 【Linux输出】【缩放比例】按较长边比较物理模式与逻辑尺寸计算一百二十分之一精度比例，缺失模式时采用整数缩放因子
    pub fn scale(&self) -> f32 {
        let physical = self
            .info
            .modes
            .iter()
            .find(|m| m.current)
            .map(|m| m.dimensions.0.max(m.dimensions.1));
        let logical = self.logical_size().map(|(w, h)| w.max(h));
        let (Some(physical), Some(logical)) = (physical, logical) else {
            return (self.info.scale_factor.max(1) as f32).clamp(0.25, 8.0);
        };
        if physical <= 0 || logical <= 0 {
            return (self.info.scale_factor.max(1) as f32).clamp(0.25, 8.0);
        }
        let scale = (physical as f32 / logical as f32 * 120.0).round() / 120.0;
        // to be sure it won't take a lot of space
        scale.clamp(0.25, 8.0)
    }
}

#[cfg(target_os = "windows")]
pub use super::windows_output::Output;

// Pipewire and pixels
#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Clone)]
pub struct Capture {
    pub pixmap: super::frozen_pixmap::FrozenPixmap,
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

#[cfg(all(test, target_os = "linux"))]
mod output_tests {
    use super::*;

    /// 【Linux输出测试】【无xdg-output回退】沙箱隐藏 xdg-output 时仍按物理模式、旋转方向和整数缩放推导逻辑尺寸与比例
    #[test]
    fn missing_xdg_output_derives_logical_size_and_scale_from_wl_output() {
        let mut info = SctkOutputInfo::x11(1, "eDP-1".into(), (0, 0), (2560, 1600));
        info.logical_size = None;
        info.scale_factor = 2;
        let output = Output {
            wl_output: None,
            info,
        };
        assert_eq!(output.logical_size(), Some((1280, 800)));
        assert_eq!(output.scale(), 2.0);

        let mut rotated = output.clone();
        rotated.info.transform = wl_output::Transform::_90;
        assert_eq!(rotated.logical_size(), Some((800, 1280)));
        assert_eq!(rotated.scale(), 2.0);
    }
}
