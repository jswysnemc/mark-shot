//! 【窗口识别】【逻辑边界】校验整数矩形并按实际几何去重

#[cfg(target_os = "linux")]
use super::Window;
use tiny_skia::Rect;

/// 【窗口识别】【矩形构造】按 x、y、width、height 构造原版整数逻辑矩形，返回有效边界或空值
pub(super) fn rect(x: f64, y: f64, width: f64, height: f64) -> Option<Rect> {
    let values = [x, y, width, height];
    if values
        .iter()
        .any(|value| !value.is_finite() || value.abs() > 1_000_000.0)
    {
        return None;
    }
    let [x, y, width, height] = values.map(f64::round_ties_even);
    if width <= 1.0 || height <= 1.0 {
        return None;
    }
    Rect::from_xywh(x as f32, y as f32, width as f32, height as f32)
}

/// 【窗口识别】【候选追加】按 windows、rect、z_order 追加有效窗口，重复矩形保留较高层级，无返回值
#[cfg(target_os = "linux")]
pub(super) fn append(windows: &mut Vec<Window>, rect: Rect, z_order: Option<i32>) {
    if let Some(window) = windows.iter_mut().find(|window| window.rect == rect) {
        window.z_order = window.z_order.max(z_order);
    } else if windows.len() < 4096 {
        windows.push(Window { rect, z_order });
    }
}
