//! 【箭头】【头部尺寸】按原版路径长度、笔宽和单双向规则约束箭头

use crate::features::line::{Point, polyline::normal};

use super::ArrowStyle;

#[derive(Clone, Copy, Debug)]
pub struct HeadMetrics {
    pub length: f32,
    pub half_width: f32,
    pub body_half_width: f32,
}

/// 【箭头】【头部尺寸】用 total_length、width、scale、style 计算头部；返回有限尺寸，绝对长度随原图比例变化
pub fn metrics(total_length: f32, width: f32, scale: f32, style: ArrowStyle) -> HeadMetrics {
    let scale = if scale.is_finite() && scale > 0.0 {
        scale.min(1_000_000.0)
    } else {
        1.0
    };
    let width = crate::features::line::clamped_width(width, scale);
    let total_length = if total_length.is_finite() && total_length > 0.0 {
        total_length
    } else {
        0.0
    };
    let body_half_width = width * 0.5;
    if style.open() {
        let length = (total_length * 0.32)
            .clamp(width * 2.6, width * 6.0)
            .min(total_length * if style.bidirectional() { 0.42 } else { 1.0 });
        HeadMetrics {
            length,
            half_width: length * 0.62,
            body_half_width,
        }
    } else {
        let length = (total_length * 0.18)
            .clamp(width * 5.0, width * 9.0)
            .clamp(12.0 * scale, 60.0 * scale)
            .min(total_length * if style.bidirectional() { 0.42 } else { 0.62 });
        HeadMetrics {
            length,
            half_width: (length * 0.28).max(body_half_width * 1.5),
            body_half_width,
        }
    }
}

/// 【箭头】【头部顶点】以 tip、base 和 direction 构造半宽 half_width 的左右基点；返回基点、尖端、基点
pub fn triangle(tip: Point, base: Point, direction: Point, half_width: f32) -> [Point; 3] {
    let normal = normal(direction);
    [
        offset(base, normal, half_width),
        tip,
        offset(base, normal, -half_width),
    ]
}

/// 【箭头】【法向偏移】返回 point 沿 direction 偏移 amount 的坐标
pub fn offset(point: Point, direction: Point, amount: f32) -> Point {
    (
        point.0 + direction.0 * amount,
        point.1 + direction.1 * amount,
    )
}
