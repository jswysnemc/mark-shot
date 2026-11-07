//! 【标注命中】【正文规则】首次选择使用骨架或旋转后外接矩形，窗口容差独立于持久原图样式

use super::chrome::{self, Point};
use crate::{
    features::{annotation_style::window::WindowUnits, ruler},
    types::{Annotation, AnnotationShape, SelectionHandle},
};
use tiny_skia::Rect;

/// 【标注命中】【位置检查】按point返回两个坐标是否均为有限值，不修改输入
fn finite(point: Point) -> bool {
    point.0.is_finite() && point.1.is_finite()
}

/// 【标注命中】【两点距离】按first与second返回欧氏距离，单位与输入坐标相同
fn distance(first: Point, second: Point) -> f32 {
    (first.0 - second.0).hypot(first.1 - second.1)
}

/// 【标注命中】【矩形正文】按bounds、point与tolerance返回扩容矩形是否包含指针，空范围拒绝命中
fn contains(bounds: Rect, point: Point, tolerance: f32) -> bool {
    point.0 >= bounds.left() - tolerance
        && point.0 <= bounds.right() + tolerance
        && point.1 >= bounds.top() - tolerance
        && point.1 <= bounds.bottom() + tolerance
}

/// 【标注命中】【圆形正文】按bounds、point与tolerance返回扩容椭圆是否包含指针，不把圆形改为矩形
fn ellipse(bounds: Rect, point: Point, tolerance: f32) -> bool {
    let center = chrome::center(bounds);
    let x = (point.0 - center.0) / (bounds.width() / 2.0 + tolerance);
    let y = (point.1 - center.1) / (bounds.height() / 2.0 + tolerance);
    x * x + y * y <= 1.0
}

/// 【标注命中】【原图选框】按annotation返回Qt生产正文选框，序号仅含圆框且不计白边，放大镜计原图笔宽，范围按冻结原图裁切
pub fn unrotated_bounds(annotation: &Annotation) -> Option<Rect> {
    let bounds = match &annotation.shape {
        AnnotationShape::Number { mark } => {
            let radius = mark.bubble_radius(annotation.stroke_width);
            Rect::from_xywh(
                mark.bubble.0 - radius,
                mark.bubble.1 - radius,
                radius * 2.0,
                radius * 2.0,
            )
        }
        AnnotationShape::Magnifier { magnifier } => magnifier
            .unrotated_bounds()?
            .outset(annotation.stroke_width, annotation.stroke_width),
        AnnotationShape::Line { line, .. } | AnnotationShape::Arrow { line, .. } => {
            let points: Vec<_> = line.points().collect();
            let width = annotation.stroke_width;
            Rect::from_ltrb(
                points.iter().map(|p| p.0).fold(f32::INFINITY, f32::min) - width,
                points.iter().map(|p| p.1).fold(f32::INFINITY, f32::min) - width,
                points.iter().map(|p| p.0).fold(f32::NEG_INFINITY, f32::max) + width,
                points.iter().map(|p| p.1).fold(f32::NEG_INFINITY, f32::max) + width,
            )
        }
        AnnotationShape::Ruler { measurement } => return measurement.unrotated_bounds(),
        _ => chrome::bounds(annotation),
    }?;
    annotation
        .frozen_bounds
        .map_or(Some(bounds), |canvas| bounds.intersect(&canvas))
}

/// 【标注命中】【本地指针】按annotation与point返回反旋转指针，专用模型使用自身轴心，其余使用生产选框中心
pub fn local_point(annotation: &Annotation, point: Point) -> Point {
    match annotation.shape {
        AnnotationShape::Number { mark } => mark.to_local(point),
        AnnotationShape::Magnifier { magnifier } => magnifier.to_local(point),
        AnnotationShape::Redact { filter } => filter.to_local(point),
        AnnotationShape::Ruler { .. } => point,
        _ => ruler::geometry::rotated(
            point,
            chrome::center(unrotated_bounds(annotation).unwrap_or(annotation.bbox)),
            -annotation.rotation_degrees as f32,
        ),
    }
}

/// 【标注命中】【可见选框】按annotation返回旋转后的轴对齐正文范围，再按冻结图像裁切，首次通用命中使用此范围
fn visual_bounds(annotation: &Annotation) -> Option<Rect> {
    let bounds = unrotated_bounds(annotation)?;
    let corners = chrome::corners(bounds).map(|point| match annotation.shape {
        AnnotationShape::Number { mark } => mark.to_visual(point),
        AnnotationShape::Magnifier { magnifier } => magnifier.to_visual(point),
        AnnotationShape::Redact { filter } => filter.to_visual(point),
        _ => ruler::geometry::rotated(
            point,
            chrome::center(bounds),
            annotation.rotation_degrees as f32,
        ),
    });
    let bounds = Rect::from_ltrb(
        corners.iter().map(|p| p.0).fold(f32::INFINITY, f32::min),
        corners.iter().map(|p| p.1).fold(f32::INFINITY, f32::min),
        corners
            .iter()
            .map(|p| p.0)
            .fold(f32::NEG_INFINITY, f32::max),
        corners
            .iter()
            .map(|p| p.1)
            .fold(f32::NEG_INFINITY, f32::max),
    )?;
    annotation
        .frozen_bounds
        .map_or(Some(bounds), |canvas| bounds.intersect(&canvas))
}

/// 【标注命中】【专用正文】按annotation、point与window返回序号圆心、尖端或放大镜控制点、内部操作；其他形状返回None
pub fn special_drag(annotation: &Annotation, point: Point, window: WindowUnits) -> SelectionHandle {
    if !finite(point) {
        return SelectionHandle::None;
    }
    let tolerance = window.canvas(8.0);
    match annotation.shape {
        AnnotationShape::Number { mark } => {
            let local = mark.to_local(point);
            let radius = mark.bubble_radius(annotation.stroke_width);
            if distance(local, mark.bubble) <= radius + tolerance {
                SelectionHandle::NumberBubble
            } else if distance(local, mark.tip) <= tolerance * 1.6 {
                SelectionHandle::NumberTip
            } else {
                SelectionHandle::None
            }
        }
        AnnotationShape::Magnifier { magnifier } => {
            // 1. 【标注命中】【双框顺序】镜框控制点优先于源框，首次点击也使用生产专用控制点
            if let Some((handle, _)) = crate::features::magnifier::drag::controls(magnifier)
                .into_iter()
                .find(|(_, control)| distance(*control, point) <= tolerance * 1.2)
            {
                return handle;
            }
            let (Some(source), Some(lens)) = (magnifier.source_rect(), magnifier.lens_rect())
            else {
                return SelectionHandle::None;
            };
            let local = magnifier.to_local(point);
            let inside = |bounds| {
                if magnifier.shape == crate::features::magnifier::MagnifierShape::Circle {
                    ellipse(bounds, local, tolerance)
                } else {
                    contains(bounds, local, 0.0)
                }
            };
            if inside(source) {
                SelectionHandle::MagnifierSource
            } else if inside(lens) {
                SelectionHandle::MagnifierLens
            } else {
                SelectionHandle::None
            }
        }
        _ => SelectionHandle::None,
    }
}

/// 【标注命中】【首次正文】按annotation、point和window返回首次选择是否命中，线条只判断骨架，其余最终判断旋转后外接范围
pub fn initial(annotation: &Annotation, point: Point, window: WindowUnits) -> bool {
    if !finite(point) {
        return false;
    }
    let tolerance = window.canvas(8.0);
    if let AnnotationShape::Ruler { measurement } = annotation.shape {
        return ruler::drag::body_hit(measurement, point, tolerance, true) != SelectionHandle::None;
    }
    if special_drag(annotation, point, window) != SelectionHandle::None {
        return true;
    }
    if let Some(line) = annotation.shape.line() {
        return crate::features::line::geometry::insertion_at(
            line,
            local_point(annotation, point),
            tolerance + annotation.stroke_width * 0.5,
        )
        .is_some();
    }
    visual_bounds(annotation).is_some_and(|bounds| contains(bounds, point, tolerance))
}

/// 【标注命中】【已选边界】按bounds、point与window返回四角、四边或移动操作，边缘使用独立十窗口像素容差
pub fn bounds_drag(bounds: Rect, point: Point, window: WindowUnits) -> SelectionHandle {
    if !finite(point) {
        return SelectionHandle::None;
    }
    let tolerance = window.canvas(10.0);
    let left = (point.0 - bounds.left()).abs() <= tolerance;
    let right = (point.0 - bounds.right()).abs() <= tolerance;
    let top = (point.1 - bounds.top()).abs() <= tolerance;
    let bottom = (point.1 - bounds.bottom()).abs() <= tolerance;
    use SelectionHandle::*;
    match (left, right, top, bottom) {
        (true, _, true, _) => TopLeft,
        (_, true, true, _) => TopRight,
        (true, _, _, true) => BottomLeft,
        (_, true, _, true) => BottomRight,
        (true, _, _, _) => Left,
        (_, true, _, _) => Right,
        (_, _, true, _) => Top,
        (_, _, _, true) => Bottom,
        _ if contains(bounds, point, tolerance) => Move,
        _ => None,
    }
}

/// 【标注命中】【已选正文】按annotation、point与window返回专用操作、最近线定点或通用边界操作；旋转把手由窗口入口优先判断
pub fn selected(annotation: &Annotation, point: Point, window: WindowUnits) -> SelectionHandle {
    if !finite(point) {
        return SelectionHandle::None;
    }
    let special = special_drag(annotation, point, window);
    if special != SelectionHandle::None {
        return special;
    }
    let local = local_point(annotation, point);
    if let Some(line) = annotation.shape.line() {
        let mut best = None;
        for (index, anchor) in line.points().enumerate() {
            let distance = distance(anchor, local);
            if distance <= window.canvas(if index < 2 { 12.0 } else { 11.2 })
                && best.is_none_or(|(_, previous)| distance < previous)
            {
                best = Some((index, distance));
            }
        }
        if let Some((index, _)) = best {
            return SelectionHandle::LinePoint(index);
        }
    }
    unrotated_bounds(annotation).map_or(SelectionHandle::None, |bounds| {
        bounds_drag(bounds, local, window)
    })
}
