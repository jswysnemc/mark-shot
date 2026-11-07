//! 【普通标注】【旋转几何】复用原版顺时针角度、固定旋转间隔和未旋转矩形坐标

use tiny_skia::{Rect, Transform};

pub type Point = (f32, f32);

/// 【普通标注】【角度归一化】按 degrees 返回介于-180与180的角度，保留正负180边界，非有限参数返回零
pub fn normalized(degrees: f64) -> f64 {
    if !degrees.is_finite() {
        return 0.0;
    }
    let degrees = degrees % 360.0;
    if degrees < -180.0 {
        degrees + 360.0
    } else if degrees > 180.0 {
        degrees - 360.0
    } else {
        degrees
    }
}

/// 【普通标注】【旋转吸附】按 degrees 返回最近45度倍数，使用原版余数规则，半圈的正负符号由最近偶数圈决定
pub fn snapped(degrees: f64) -> f64 {
    if !degrees.is_finite() {
        return 0.0;
    }
    let rounded = (degrees / 45.0).round() * 45.0;
    rounded - (rounded / 360.0).round_ties_even() * 360.0
}

/// 【普通标注】【旋转增量】按 center、start、current 和 minimum_radius 返回顺时针角度增量，方向不可靠时返回空值
pub fn drag_degrees(
    center: (f64, f64),
    start: (f64, f64),
    current: (f64, f64),
    minimum_radius: f64,
) -> Option<f64> {
    let vector = |point: (f64, f64)| (point.0 - center.0, point.1 - center.1);
    let first = vector(start);
    let next = vector(current);
    if !first.0.is_finite()
        || !first.1.is_finite()
        || !next.0.is_finite()
        || !next.1.is_finite()
        || first.0.hypot(first.1) <= minimum_radius
        || next.0.hypot(next.1) <= minimum_radius
    {
        return None;
    }
    Some((next.1.atan2(next.0) - first.1.atan2(first.0)).to_degrees())
}

/// 【普通标注】【矩形中心】按 bounds 返回未旋转几何中心，无额外描边留量
pub fn center(bounds: Rect) -> Point {
    (
        (bounds.left() + bounds.right()) * 0.5,
        (bounds.top() + bounds.bottom()) * 0.5,
    )
}

/// 【普通标注】【旋转坐标】按 point、center 和 degrees 返回旋转后的逻辑点，零角度保持原始坐标
pub fn rotated(point: Point, center: Point, degrees: f64) -> Point {
    let degrees = normalized(degrees);
    if degrees == 0.0 {
        return point;
    }
    let (sin, cos) = degrees.to_radians().sin_cos();
    let dx = f64::from(point.0) - f64::from(center.0);
    let dy = f64::from(point.1) - f64::from(center.1);
    (
        (f64::from(center.0) + dx * cos - dy * sin) as f32,
        (f64::from(center.1) + dx * sin + dy * cos) as f32,
    )
}

/// 【普通标注】【旋转外框】按 bounds、center 和 degrees 返回顺时针四角，保留未旋转控制点顺序
pub fn corners(bounds: Rect, center: Point, degrees: f64) -> [Point; 4] {
    crate::features::annotation_selection::chrome::corners(bounds)
        .map(|point| rotated(point, center, degrees))
}

/// 【普通标注】【轴对齐边界】按 bounds、center 和 degrees 返回旋转四角的轴对齐范围，退化时保留输入矩形
pub fn bounds(bounds: Rect, center: Point, degrees: f64) -> Rect {
    if normalized(degrees) == 0.0 {
        return bounds;
    }
    let corners = corners(bounds, center, degrees);
    Rect::from_ltrb(
        corners
            .iter()
            .map(|point| point.0)
            .fold(f32::INFINITY, f32::min),
        corners
            .iter()
            .map(|point| point.1)
            .fold(f32::INFINITY, f32::min),
        corners
            .iter()
            .map(|point| point.0)
            .fold(f32::NEG_INFINITY, f32::max),
        corners
            .iter()
            .map(|point| point.1)
            .fold(f32::NEG_INFINITY, f32::max),
    )
    .unwrap_or(bounds)
}

/// 【普通标注】【绘制矩阵】按 center 和 degrees 返回逻辑坐标旋转矩阵，缩放与画布偏移交给调用方
pub fn transform(center: Point, degrees: f64) -> Transform {
    let (sin, cos) = normalized(degrees).to_radians().sin_cos();
    let (x, y) = (f64::from(center.0), f64::from(center.1));
    Transform::from_row(
        cos as f32,
        sin as f32,
        -sin as f32,
        cos as f32,
        (x - x * cos + y * sin) as f32,
        (y - x * sin - y * cos) as f32,
    )
}

/// 【普通标注】【旋转控制点】按 bounds、center 和 degrees 返回上边中点及外移26逻辑像素的旋转点
pub fn handle(bounds: Rect, center: Point, degrees: f64) -> (Point, Point) {
    let top = rotated((self::center(bounds).0, bounds.top()), center, degrees);
    let vector = (top.0 - center.0, top.1 - center.1);
    let length = vector.0.hypot(vector.1);
    let direction = if length <= 0.1 {
        (0.0, -1.0)
    } else {
        (vector.0 / length, vector.1 / length)
    };
    (
        top,
        (top.0 + direction.0 * 26.0, top.1 + direction.1 * 26.0),
    )
}
