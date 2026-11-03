//! 【放大镜】【编辑控制点】使用统一操作颜色绘制双框控制点，不进入导出图像

use tiny_skia::{Color, PathBuilder, Stroke, StrokeDash};

use super::Painter;
use crate::features::magnifier::{drag, draw};
use crate::types::{Annotation, AnnotationShape};

/// 【放大镜】【双框控制点绘制】把 annotation 的两组控制点和旋转把手绘入 painter；仅用于编辑屏幕，无返回值
pub(crate) fn draw(painter: &mut Painter<'_>, annotation: &Annotation) {
    let AnnotationShape::Magnifier { magnifier } = annotation.shape else {
        return;
    };
    let color = crate::config::get().annotations.handles.color.get().color();
    let shadow = Color::from_rgba8(8, 13, 19, 230);
    let unit = magnifier.style_scale;
    for rect in [magnifier.lens_rect(), magnifier.source_rect()]
        .into_iter()
        .flatten()
    {
        if let Some(path) = draw::frame_path(magnifier, rect) {
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
    }
    let mut points = drag::controls(magnifier)
        .into_iter()
        .map(|(_, point)| point)
        .collect::<Vec<_>>();
    if let Some(handle) = drag::rotation_handle(magnifier)
        && let Some(bounds) = magnifier.unrotated_bounds()
    {
        let start = magnifier.to_visual(((bounds.left() + bounds.right()) / 2.0, bounds.top()));
        let mut path = PathBuilder::new();
        path.move_to(start.0, start.1);
        path.line_to(handle.0, handle.1);
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
                    width: unit,
                    ..Stroke::default()
                },
            );
        }
        points.push(handle);
    }
    for point in points {
        painter.fill_circle(point.0, point.1, 4.0 * unit, shadow);
        painter.fill_circle(point.0, point.1, 2.5 * unit, color);
    }
}
