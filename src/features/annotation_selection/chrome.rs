//! 【标注选框】【操作几何】统一控制点尺寸与删除按钮位置，使用窗口逻辑坐标

use tiny_skia::{Color, Rect};

pub type Point = (f32, f32);

/// 【标注选框】【矩形中心】按 rect 返回几何中心，不分配对象
pub fn center(rect: Rect) -> Point {
    (
        (rect.left() + rect.right()) * 0.5,
        (rect.top() + rect.bottom()) * 0.5,
    )
}

/// 【标注选框】【原始边界】按 annotation 返回原版选框所用正文边界，矩形不包括描边光晕，笔迹包含笔宽
pub fn bounds(annotation: &crate::types::Annotation) -> Option<Rect> {
    use crate::types::AnnotationShape;
    let bounds = match &annotation.shape {
        AnnotationShape::Arrow { .. } | AnnotationShape::Line { .. } => {
            super::body_hit::unrotated_bounds(annotation)
        }
        AnnotationShape::Rectangle { start, end, .. }
        | AnnotationShape::Circle { start, end, .. }
        | AnnotationShape::Marker { start, end, .. } => Rect::from_ltrb(
            start.0.min(end.0),
            start.1.min(end.1),
            start.0.max(end.0),
            start.1.max(end.1),
        ),
        AnnotationShape::Redact { filter } => filter.rect(),
        AnnotationShape::Number { mark } => mark.unrotated_bounds(annotation.stroke_width),
        AnnotationShape::Magnifier { magnifier } => magnifier.unrotated_bounds(),
        AnnotationShape::Ruler { measurement } => measurement.unrotated_bounds(),
        AnnotationShape::Pen { points } | AnnotationShape::Highlighter { points, .. } => {
            let pad = annotation.stroke_width;
            Rect::from_ltrb(
                points.iter().map(|p| p.0).fold(f32::INFINITY, f32::min) - pad,
                points.iter().map(|p| p.1).fold(f32::INFINITY, f32::min) - pad,
                points.iter().map(|p| p.0).fold(f32::NEG_INFINITY, f32::max) + pad,
                points.iter().map(|p| p.1).fold(f32::NEG_INFINITY, f32::max) + pad,
            )
        }
        _ => Some(annotation.bbox),
    }?;
    if annotation.uses_generic_rotation()
        && let Some(canvas) = annotation.frozen_bounds
    {
        bounds.intersect(&canvas)
    } else {
        Some(bounds)
    }
}

/// 【标注选框】【操作颜色】返回原版固定橙色，无参数，不随界面主题改变
pub fn color() -> Color {
    Color::from_rgba8(251, 146, 60, 255)
}

/// 【标注选框】【矩形角点】按 rect 返回顺时针四角，无额外命中留白
pub fn corners(rect: Rect) -> [Point; 4] {
    [
        (rect.left(), rect.top()),
        (rect.right(), rect.top()),
        (rect.right(), rect.bottom()),
        (rect.left(), rect.bottom()),
    ]
}

/// 【标注选框】【缩放位置】按 corners 返回四角与四边中点，不改变旋转后的坐标
pub fn handles(corners: [Point; 4]) -> [Point; 8] {
    let midpoint = |a: Point, b: Point| ((a.0 + b.0) * 0.5, (a.1 + b.1) * 0.5);
    [
        corners[0],
        midpoint(corners[0], corners[1]),
        corners[1],
        midpoint(corners[1], corners[2]),
        corners[2],
        midpoint(corners[2], corners[3]),
        corners[3],
        midpoint(corners[3], corners[0]),
    ]
}

/// 【标注选框】【删除位置】按 frame、corner、radial 与 size 返回20像素按钮，旋转单选沿右上角方向外移24像素，并保留8像素窗口边距
pub fn delete_rect(frame: Rect, corner: Point, radial: bool, size: (f32, f32)) -> Rect {
    let mut center = (
        f64::from(frame.right()) + 18.0,
        f64::from(frame.top()) - 18.0,
    );
    if radial {
        let direction = (
            f64::from(corner.0) - f64::from(self::center(frame).0),
            f64::from(corner.1) - f64::from(self::center(frame).1),
        );
        let length = direction.0.hypot(direction.1);
        let direction = if length <= 0.1 {
            (1.0, -1.0)
        } else {
            (direction.0 / length, direction.1 / length)
        };
        center = (
            f64::from(corner.0) + direction.0 * 24.0,
            f64::from(corner.1) + direction.1 * 24.0,
        );
    }
    Rect::from_xywh(
        (center.0 - 10.0).clamp(8.0, (f64::from(size.0) - 28.0).max(8.0)) as f32,
        (center.1 - 10.0).clamp(8.0, (f64::from(size.1) - 28.0).max(8.0)) as f32,
        20.0,
        20.0,
    )
    .unwrap()
}
