//! 【标注样式】【绘制范围】窗口限制只用于绘制和损坏范围，不覆盖原图宽度或持久几何

use crate::{
    features::annotation_style::window::WindowUnits,
    types::{Annotation, AnnotationShape},
};
use tiny_skia::Rect;

/// 【标注样式】【实际损坏】按annotation与window返回包含窗口最小尺寸的损坏范围，保证局部剔除保留实际正文
pub(super) fn damage(
    annotation: &Annotation,
    window: WindowUnits,
    allowed: Option<Rect>,
    projection: crate::editor::canvas_navigation::transform::ViewTransform,
) -> Rect {
    let stored = annotation.damage_bbox(false);
    if let AnnotationShape::Ruler { measurement } = annotation.shape {
        return super::ruler_chrome::bounds(
            measurement,
            annotation.stroke_width,
            allowed,
            window,
            projection,
        )
        .unwrap_or(stored);
    }
    if let AnnotationShape::Number { mark } = annotation.shape {
        return crate::features::number::window_geometry::Geometry::new(
            mark,
            annotation.stroke_width,
            window,
        )
        .painted_bounds(window)
        .and_then(|bounds| bounds.outset(window.canvas(2.0), window.canvas(2.0)))
        .and_then(|bounds| {
            Rect::from_ltrb(
                stored.left().min(bounds.left()),
                stored.top().min(bounds.top()),
                stored.right().max(bounds.right()),
                stored.bottom().max(bounds.bottom()),
            )
        })
        .unwrap_or(stored);
    }
    let minimum = if matches!(annotation.shape, AnnotationShape::Highlighter { .. }) {
        6.0
    } else {
        1.5
    };
    let mut extra =
        (window.canvas(minimum) - annotation.stroke_width).max(0.0) / 2.0 + window.canvas(2.0);
    if matches!(annotation.shape, AnnotationShape::Arrow { .. }) {
        extra = extra.max(window.canvas(20.0));
    }
    stored.outset(extra, extra).unwrap_or(stored)
}
