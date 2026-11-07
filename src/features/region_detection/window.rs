//! 【窗口识别】【层级命中】明确层级优先，没有层级时回退到较小嵌套窗口

use super::PixelRect;

pub struct Window {
    pub rect: PixelRect,
    pub z_order: Option<i32>,
}

/// 【窗口识别】【最前窗口】按 windows 与 point 选择覆盖该位置的最前窗口，返回矩形，没有命中时返回空值
pub fn topmost(windows: &[Window], point: (i32, i32)) -> Option<PixelRect> {
    let mut candidates = windows.iter().filter(|window| window.rect.contains(point));
    let mut best = candidates.next()?;
    let has_z = windows
        .iter()
        .any(|window| window.rect.contains(point) && window.z_order.is_some());
    for candidate in candidates {
        let preferred = if has_z {
            candidate.z_order.unwrap_or(i32::MIN) > best.z_order.unwrap_or(i32::MIN)
        } else {
            candidate.rect.area() < best.rect.area()
        };
        if preferred {
            best = candidate;
        }
    }
    Some(best.rect)
}
