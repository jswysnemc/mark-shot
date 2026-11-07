//! 【截图裁切】【物理边缘】先映射物理像素再扩展边缘，只消除单精度视口换算产生的整数邻近误差

use tiny_skia::{IntRect, Rect};

/// 【截图裁切】【整数邻近】按 value 和 logical_magnitude 消除 scale 换算误差，返回稳定边缘，真实分数坐标保持不变
fn stable(value: f64, logical_magnitude: f64, scale: f64) -> f64 {
    let rounded = value.round();
    let tolerance = logical_magnitude.max(1.0) * scale * f64::from(f32::EPSILON) * 2.0;
    if (value - rounded).abs() <= tolerance.min(0.01) {
        rounded
    } else {
        value
    }
}

/// 【截图裁切】【物理范围】按 region、origin 和 scale 返回对齐像素矩形，空范围或无效比例返回空值，不提前扩展逻辑边缘
pub(crate) fn map(region: Rect, origin: (f32, f32), scale: f64) -> Option<IntRect> {
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    let edge = |value: f32, origin: f32| {
        stable(
            (f64::from(value) - f64::from(origin)) * scale,
            f64::from(value.abs().max(origin.abs())),
            scale,
        )
    };
    let left = edge(region.left(), origin.0).floor() as i32;
    let top = edge(region.top(), origin.1).floor() as i32;
    let right = edge(region.right(), origin.0).ceil() as i32;
    let bottom = edge(region.bottom(), origin.1).ceil() as i32;
    IntRect::from_xywh(
        left,
        top,
        right.checked_sub(left)? as u32,
        bottom.checked_sub(top)? as u32,
    )
}
