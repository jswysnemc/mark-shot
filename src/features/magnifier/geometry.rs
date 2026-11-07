//! 【放大镜】【双框几何】负责原图约束、旋转坐标和连接线，不读取像素

use tiny_skia::Rect;

use super::model::{MAX_DIAMETER, MIN_DIAMETER, MIN_DRAG_DISTANCE};
use super::{Magnifier, MagnifierShape, Point};

/// 【放大镜】【坐标检查】返回 point 的两个坐标是否均为有限值
pub fn finite(point: Point) -> bool {
    point.0.is_finite() && point.1.is_finite()
}

/// 【放大镜】【框中心】用 center、size 与 bounds 返回可容纳完整框的中心；超出原图尺寸的轴使用原图中心
pub fn clamped_center(center: Point, size: Point, bounds: Rect) -> Point {
    let axis = |value: f32, size: f32, start: f32, end: f32| {
        let half = size.max(0.0) / 2.0;
        if half * 2.0 >= end - start {
            (start + end) / 2.0
        } else {
            value.clamp(start + half, end - half)
        }
    };
    (
        axis(center.0, size.0, bounds.left(), bounds.right()),
        axis(center.1, size.1, bounds.top(), bounds.bottom()),
    )
}

/// 【放大镜】【中心矩形】用 center、size 构造矩形；空尺寸或非有限坐标返回空值
pub fn centered_rect(center: Point, size: Point) -> Option<Rect> {
    if !finite(center) || !finite(size) {
        return None;
    }
    Rect::from_xywh(
        center.0 - size.0 / 2.0,
        center.1 - size.1 / 2.0,
        size.0,
        size.1,
    )
    .filter(|rect| !rect.is_empty())
}

/// 【放大镜】【点旋转】把 point 绕 pivot 旋转 radians 弧度；返回新坐标
pub fn rotated(point: Point, pivot: Point, radians: f32) -> Point {
    let (sin, cos) = radians.sin_cos();
    let delta = (point.0 - pivot.0, point.1 - pivot.1);
    (
        pivot.0 + delta.0 * cos - delta.1 * sin,
        pivot.1 + delta.0 * sin + delta.1 * cos,
    )
}

/// 【放大镜】【矩形角点】返回 rect 的左上、右上、右下和左下坐标
pub fn corners(rect: Rect) -> [Point; 4] {
    [
        (rect.left(), rect.top()),
        (rect.right(), rect.top()),
        (rect.right(), rect.bottom()),
        (rect.left(), rect.bottom()),
    ]
}

impl Magnifier {
    /// 【放大镜】【镜框读取】返回未旋转的镜框矩形；空草稿返回空值
    pub fn lens_rect(self) -> Option<Rect> {
        centered_rect(self.center, self.size)
    }

    /// 【放大镜】【源框读取】按固定倍率返回原图内的完整源框，圆形采用镜框短边，不改写请求源点，空草稿返回空值
    pub fn source_rect(self) -> Option<Rect> {
        let size = if self.shape == MagnifierShape::Circle {
            let diameter = self.size.0.min(self.size.1) / self.zoom;
            (diameter, diameter)
        } else {
            (
                (self.size.0 / self.zoom).min(self.image_bounds.width()),
                (self.size.1 / self.zoom).min(self.image_bounds.height()),
            )
        };
        centered_rect(clamped_center(self.source, size, self.image_bounds), size)
    }

    /// 【放大镜】【尺寸有效性】返回镜框两边是否均达到四个原图像素，不把空手势提交为标注
    pub fn measures(self) -> bool {
        self.size.0 * self.pixel_scale >= 4.0 && self.size.1 * self.pixel_scale >= 4.0
    }

    /// 【放大镜】【整体约束】返回同时满足原图范围和固定倍率的双框；只调整超出原图的尺寸与中心
    pub fn constrained(self) -> Self {
        let mut result = self;
        result.size.0 = result.size.0.max(0.0).min(self.image_bounds.width());
        result.size.1 = result.size.1.max(0.0).min(self.image_bounds.height());
        if result.shape == MagnifierShape::Circle {
            let side = result.size.0.min(result.size.1);
            result.size = (side, side);
        }
        result.center = clamped_center(result.center, result.size, result.image_bounds);
        result.source = clamped_center(
            result.source,
            (result.size.0 / result.zoom, result.size.1 / result.zoom),
            result.image_bounds,
        );
        result
    }

    /// 【放大镜】【拖动草稿】以 point 作为镜框中心决定直径；返回新草稿，短于六十四个原图像素时保留空尺寸
    pub fn draft_to(self, point: Point) -> Self {
        if !finite(point) {
            return self;
        }
        let point = (
            point
                .0
                .clamp(self.image_bounds.left(), self.image_bounds.right()),
            point
                .1
                .clamp(self.image_bounds.top(), self.image_bounds.bottom()),
        );
        let distance = (point.0 - self.source.0).hypot(point.1 - self.source.1);
        let mut result = Self {
            center: point,
            size: (0.0, 0.0),
            ..self
        };
        if distance * self.pixel_scale < MIN_DRAG_DISTANCE {
            return result;
        }
        let max = (MAX_DIAMETER / self.pixel_scale)
            .min(self.image_bounds.width())
            .min(self.image_bounds.height());
        let min = (MIN_DIAMETER / self.pixel_scale).min(max);
        let diameter = (distance * 1.05).clamp(min, max);
        result.size = (diameter, diameter);
        result = result.constrained();
        if let Some(bounds) = result.unrotated_bounds() {
            result.pivot = (
                (bounds.left() + bounds.right()) / 2.0,
                (bounds.top() + bounds.bottom()) / 2.0,
            );
        }
        result
    }

    /// 【放大镜】【基础边界】返回未旋转双框的联合边界；空草稿返回空值
    pub fn unrotated_bounds(self) -> Option<Rect> {
        self.lens_rect()?.join(&self.source_rect()?)
    }

    /// 【放大镜】【显示坐标】将未旋转 point 映射到屏幕或导出坐标；返回旋转后的点
    pub fn to_visual(self, point: Point) -> Point {
        rotated(point, self.pivot, self.rotation)
    }

    /// 【放大镜】【局部坐标】将屏幕或导出的 point 反旋转到双框坐标；返回未旋转的点
    pub fn to_local(self, point: Point) -> Point {
        rotated(point, self.pivot, -self.rotation)
    }

    /// 【放大镜】【正文边界】返回旋转后完整双框的边界，包含位于双框之间的连接线；空草稿返回空值
    pub fn bounds(self) -> Option<Rect> {
        let mut bounds = None;
        for rect in [self.lens_rect()?, self.source_rect()?] {
            let frame = if self.shape == MagnifierShape::Circle {
                centered_rect(
                    self.to_visual((
                        (rect.left() + rect.right()) / 2.0,
                        (rect.top() + rect.bottom()) / 2.0,
                    )),
                    (rect.width(), rect.height()),
                )?
            } else {
                let points = corners(rect).map(|point| self.to_visual(point));
                Rect::from_ltrb(
                    points
                        .iter()
                        .map(|point| point.0)
                        .fold(f32::INFINITY, f32::min),
                    points
                        .iter()
                        .map(|point| point.1)
                        .fold(f32::INFINITY, f32::min),
                    points
                        .iter()
                        .map(|point| point.0)
                        .fold(f32::NEG_INFINITY, f32::max),
                    points
                        .iter()
                        .map(|point| point.1)
                        .fold(f32::NEG_INFINITY, f32::max),
                )?
            };
            bounds =
                Some(bounds.map_or(frame, |before: Rect| before.join(&frame).unwrap_or(before)));
        }
        bounds
    }

    /// 【放大镜】【双连接线】返回原版三十四度取点的两条连接线；双框接近重叠时返回空值
    pub fn connectors(self) -> Option<[(Point, Point); 2]> {
        let source = self.source_rect()?;
        let lens = self.lens_rect()?;
        let source_center = (
            (source.left() + source.right()) / 2.0,
            (source.top() + source.bottom()) / 2.0,
        );
        let delta = (
            self.center.0 - source_center.0,
            self.center.1 - source_center.1,
        );
        let length = delta.0.hypot(delta.1);
        let (source_radius, lens_radius) = (source.width() / 2.0, lens.width() / 2.0);
        if length <= lens_radius + source_radius * 0.65 {
            return None;
        }
        let unit = (delta.0 / length, delta.1 / length);
        let normal = (-unit.1, unit.0);
        let (across, along) = 34.0_f32.to_radians().sin_cos();
        Some([1.0, -1.0].map(|sign| {
            let source = (
                source_center.0
                    + unit.0 * source_radius * along
                    + normal.0 * source_radius * across * sign,
                source_center.1
                    + unit.1 * source_radius * along
                    + normal.1 * source_radius * across * sign,
            );
            let lens = (
                self.center.0 - unit.0 * lens_radius * along
                    + normal.0 * lens_radius * across * sign,
                self.center.1 - unit.1 * lens_radius * along
                    + normal.1 * lens_radius * across * sign,
            );
            (self.to_visual(source), self.to_visual(lens))
        }))
    }
}
