//! 【序号】【编辑控制框】绘制独立圆心、尖端、外框与旋转把手，不进入导出

use tiny_skia::{Color, PathBuilder, Stroke, StrokeDash};

use super::Painter;
use crate::features::number::drag;
use crate::types::{Annotation, AnnotationShape, SelectionHandle};

/// 【序号】【控制框绘制】将 annotation 的编辑控制点绘入 painter，复用统一颜色；无有效序号时不绘制，无返回值
pub(crate) fn draw(painter: &mut Painter<'_>, annotation: &Annotation) {
    let AnnotationShape::Number { mark } = annotation.shape else {
        return;
    };
    let points = drag::controls(mark, annotation.stroke_width);
    if points.len() != 10 {
        return;
    }
    let unit = mark.style_scale;
    let color = crate::config::get().annotations.handles.color.get().color();
    let shadow = Color::from_rgba8(8, 13, 19, 230);
    let mut path = PathBuilder::new();
    path.move_to(points[0].1.0, points[0].1.1);
    for index in [2, 7, 5] {
        path.line_to(points[index].1.0, points[index].1.1);
    }
    path.close();
    if let Some(path) = path.finish() {
        painter.stroke_path(
            &path,
            shadow,
            &Stroke {
                width: 3.0 * unit,
                ..Stroke::default()
            },
        );
        painter.stroke_path(
            &path,
            color,
            &Stroke {
                width: 1.2 * unit,
                dash: StrokeDash::new(vec![4.0 * unit, 3.0 * unit], 0.0),
                ..Stroke::default()
            },
        );
    }
    if let Some(handle) = drag::rotation_handle(mark, annotation.stroke_width) {
        let mut line = PathBuilder::new();
        line.move_to(points[1].1.0, points[1].1.1);
        line.line_to(handle.0, handle.1);
        if let Some(line) = line.finish() {
            painter.stroke_path(
                &line,
                shadow,
                &Stroke {
                    width: 3.0 * unit,
                    ..Stroke::default()
                },
            );
            painter.stroke_path(
                &line,
                color,
                &Stroke {
                    width: unit,
                    ..Stroke::default()
                },
            );
        }
        painter.fill_circle(handle.0, handle.1, 4.0 * unit, shadow);
        painter.fill_circle(handle.0, handle.1, 2.5 * unit, color);
    }
    for (handle, point) in points {
        let radius = if matches!(
            handle,
            SelectionHandle::NumberBubble | SelectionHandle::NumberTip
        ) {
            5.0 * unit
        } else {
            4.0 * unit
        };
        painter.fill_circle(point.0, point.1, radius, shadow);
        painter.fill_circle(point.0, point.1, radius - 1.5 * unit, color);
    }
}
