//! 【尺子】【测量几何】计算像素测量、夹角、旋转和非退化边界

use tiny_skia::Rect;

use super::{Measurement, Point, RulerGeometry};

/// 【尺子】【点间距离】返回 first 与 second 的欧氏距离，单位与输入坐标一致
pub fn distance(first: Point, second: Point) -> f32 {
    (second.0 - first.0).hypot(second.1 - first.1)
}

/// 【尺子】【矩形中心】返回 rect 的几何中心，单位与输入矩形一致
pub fn rect_center(rect: Rect) -> Point {
    (
        (rect.left() + rect.right()) / 2.0,
        (rect.top() + rect.bottom()) / 2.0,
    )
}

/// 【尺子】【矩形命中】判断 point 是否位于 rect 边界以内，返回布尔值
pub fn contains(rect: Rect, point: Point) -> bool {
    point.0 >= rect.left()
        && point.0 <= rect.right()
        && point.1 >= rect.top()
        && point.1 <= rect.bottom()
}

/// 【尺子】【矩形重叠】判断 first、second 是否具有正面积交集；只接触边界不算重叠
pub fn overlaps(first: Rect, second: Rect) -> bool {
    first.left() < second.right()
        && first.right() > second.left()
        && first.top() < second.bottom()
        && first.bottom() > second.top()
}

/// 【尺子】【定点旋转】将 point 绕 center 顺时针旋转 degrees；返回旋转后坐标
pub fn rotated(point: Point, center: Point, degrees: f32) -> Point {
    let (sin, cos) = degrees.to_radians().sin_cos();
    let delta = (point.0 - center.0, point.1 - center.1);
    (
        center.0 + delta.0 * cos - delta.1 * sin,
        center.1 + delta.0 * sin + delta.1 * cos,
    )
}

/// 【尺子】【较小夹角】使用 vertex、first、second 计算 0 至 180 度的夹角；退化边返回 0
pub fn angle_degrees(vertex: Point, first: Point, second: Point) -> f32 {
    let a = (first.0 - vertex.0, first.1 - vertex.1);
    let b = (second.0 - vertex.0, second.1 - vertex.1);
    if a.0.hypot(a.1) <= 0.0 || b.0.hypot(b.1) <= 0.0 {
        return 0.0;
    }
    (a.0 * b.1 - a.1 * b.0)
        .abs()
        .atan2(a.0 * b.0 + a.1 * b.1)
        .to_degrees()
}

/// 【尺子】【点集边界】计算 points 外接矩形；退化轴按 minimum 补足，非法或空点集返回空值
pub fn points_bounds(points: &[Point], minimum: f32) -> Option<Rect> {
    let mut left = f32::INFINITY;
    let mut right = f32::NEG_INFINITY;
    let mut top = f32::INFINITY;
    let mut bottom = f32::NEG_INFINITY;
    for &(x, y) in points {
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        left = left.min(x);
        right = right.max(x);
        top = top.min(y);
        bottom = bottom.max(y);
    }
    let width = (right - left).max(minimum.max(f32::EPSILON));
    let height = (bottom - top).max(minimum.max(f32::EPSILON));
    Rect::from_xywh(
        (left + right - width) / 2.0,
        (top + bottom - height) / 2.0,
        width,
        height,
    )
}

impl Measurement {
    /// 【尺子】【有效测量】返回长度或角度是否达到原图像素最小值；角度还要求三个完整定点
    pub fn measures(self) -> bool {
        match self.geometry {
            RulerGeometry::Length { start, end, .. } => {
                (end.0 - start.0).abs() * self.pixel_scale >= 1.0
                    || (end.1 - start.1).abs() * self.pixel_scale >= 1.0
            }
            RulerGeometry::Angle { points, count } => {
                count == 3
                    && distance(points[0], points[1]) * self.pixel_scale >= 1.0
                    && distance(points[0], points[2]) * self.pixel_scale >= 1.0
            }
        }
    }

    /// 【尺子】【未旋转范围】返回当前逻辑几何外接矩形；长度尺不应用旋转，退化轴补足一个原图像素
    pub fn unrotated_bounds(self) -> Option<Rect> {
        let minimum = 1.0 / self.pixel_scale;
        match self.geometry {
            RulerGeometry::Length { start, end, .. } => points_bounds(&[start, end], minimum),
            RulerGeometry::Angle { points, count } => {
                points_bounds(&points[..usize::from(count.min(3))], minimum)
            }
        }
    }

    /// 【尺子】【长度角点】返回长度尺四个旋转后角点，按左上、右上、右下、左下排列；角度尺返回空值
    pub fn corners(self) -> Option<[Point; 4]> {
        let RulerGeometry::Length { rotation, .. } = self.geometry else {
            return None;
        };
        let bounds = self.unrotated_bounds()?;
        let center = rect_center(bounds);
        Some(
            [
                (bounds.left(), bounds.top()),
                (bounds.right(), bounds.top()),
                (bounds.right(), bounds.bottom()),
                (bounds.left(), bounds.bottom()),
            ]
            .map(|point| rotated(point, center, rotation)),
        )
    }

    /// 【尺子】【可见几何】返回旋转后的测量范围；刻度、标签与操作把手由对应模块增加留量
    pub fn bounds(self) -> Option<Rect> {
        match self.corners() {
            Some(corners) => points_bounds(&corners, 1.0 / self.pixel_scale),
            None => self.unrotated_bounds(),
        }
    }

    /// 【尺子】【长度数据】返回原图像素宽、高、对角线与面积；角度尺返回空值
    pub fn length_metrics(self) -> Option<(u64, u64, u64, u64)> {
        let RulerGeometry::Length { start, end, .. } = self.geometry else {
            return None;
        };
        let width = (end.0 - start.0).abs() * self.pixel_scale;
        let height = (end.1 - start.1).abs() * self.pixel_scale;
        let rounded_width = width.round() as u64;
        let rounded_height = height.round() as u64;
        Some((
            rounded_width,
            rounded_height,
            width.hypot(height).round() as u64,
            rounded_width.saturating_mul(rounded_height),
        ))
    }

    /// 【尺子】【测量说明】返回长度说明或保留一位小数的角度说明；未完成测量返回空值
    pub fn info_text(self) -> Option<String> {
        if !self.measures() {
            return None;
        }
        match self.geometry {
            RulerGeometry::Length { .. } => {
                let (width, height, diagonal, area) = self.length_metrics()?;
                Some(format!(
                    "{width} x {height} px   diag {diagonal} px   area {area} px"
                ))
            }
            RulerGeometry::Angle { points, .. } => Some(format!(
                "{:.1}\u{b0}",
                angle_degrees(points[0], points[1], points[2])
            )),
        }
    }

    /// 【尺子】【几何旋转】绕测量中心旋转 degrees；返回新测量，长度尺寸与角度夹角保持不变
    pub fn rotated(self, degrees: f32) -> Self {
        if !degrees.is_finite() {
            return self;
        }
        match self.geometry {
            RulerGeometry::Length {
                start,
                end,
                rotation,
            } => Self {
                geometry: RulerGeometry::Length {
                    start,
                    end,
                    rotation: (rotation + degrees + 180.0).rem_euclid(360.0) - 180.0,
                },
                ..self
            },
            RulerGeometry::Angle { .. } => {
                let Some(bounds) = self.bounds() else {
                    return self;
                };
                let center = rect_center(bounds);
                self.mapped(|point| rotated(point, center, degrees))
            }
        }
    }
}
