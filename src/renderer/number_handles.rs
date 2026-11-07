//! 【序号】【编辑控制框】共享选框样式，圆心和尖端保留原版独立控制点

use super::Painter;
use crate::{
    features::{annotation_selection::chrome, number::drag},
    types::{Annotation, AnnotationShape, SelectionHandle},
    ui::annotation_selection::draw,
};
use tiny_skia::Color;

/// 【序号】【控制框绘制】按 annotation 绘制外框、八个缩放点及圆心尖端，无返回值，不修改正文
pub(crate) fn draw(painter: &mut Painter<'_>, annotation: &Annotation) {
    let AnnotationShape::Number { mark } = annotation.shape else {
        return;
    };
    let points = drag::controls(mark, annotation.stroke_width);
    if points.len() != 10 {
        return;
    }
    let unit = 1.0;
    draw::frame(
        painter,
        [points[0].1, points[2].1, points[7].1, points[5].1],
        unit,
    );
    if let Some((start, handle)) =
        crate::features::annotation_selection::rotation::connector(annotation)
    {
        draw::rotation(painter, start, handle, unit);
    }
    for (handle, point) in points {
        match handle {
            SelectionHandle::NumberBubble => draw::circle(
                painter,
                point,
                6.0,
                unit,
                chrome::color(),
                Color::WHITE,
                2.0,
            ),
            SelectionHandle::NumberTip => draw::circle(
                painter,
                point,
                5.5,
                unit,
                Color::WHITE,
                chrome::color(),
                2.0,
            ),
            _ => draw::square(painter, point, unit),
        }
    }
}
