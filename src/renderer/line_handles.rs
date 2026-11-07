//! 【线条】【骨架控制框】复用统一外框，绘制端点、外部控制点、旋转及当前定点提示

use crate::{features::annotation_selection::chrome, ui::annotation_selection::draw};
use tiny_skia::Color;

use super::Painter;
use crate::features::line::drag;
use crate::types::Annotation;

/// 【线条】【控制框绘制】将 annotation 的外框和定点绘入 painter；不修改正文或历史，无返回值
pub(crate) fn draw(painter: &mut Painter<'_>, annotation: &Annotation) {
    let Some(line) = annotation.shape.line() else {
        return;
    };
    let Some(frame) = chrome::bounds(annotation).and_then(drag::frame) else {
        return;
    };
    super::annotations::draw_exact_frame_handles(painter, &frame);
    let rotation = drag::rotation_point(line, frame);
    draw::rotation(painter, (rotation.0, frame.top()), rotation, 1.0);
    anchors(painter, annotation);
}

/// 【线条】【统一定点】按annotation把未旋转或已旋转骨架定点绘入painter，端点六像素、内部定点五点五像素，边宽两像素，无返回值
pub(crate) fn anchors(painter: &mut Painter<'_>, annotation: &Annotation) {
    let Some(line) = annotation.shape.line() else {
        return;
    };
    let color = chrome::color();
    for (index, point) in line.points().enumerate() {
        draw::circle(
            painter,
            annotation.to_visual(point),
            if index < 2 { 6.0 } else { 5.5 },
            1.0,
            if index < 2 { color } else { Color::WHITE },
            if index < 2 { Color::WHITE } else { color },
            2.0,
        );
    }
}

/// 【线条】【当前定点提示】按annotation与index把内部定点放大到六点五像素并绘入painter，端点与越界索引不绘制提示，无返回值
pub(crate) fn highlight(painter: &mut Painter<'_>, annotation: &Annotation, index: usize) {
    if index < 2 {
        return;
    }
    let Some(point) = annotation.shape.line().and_then(|line| line.point(index)) else {
        return;
    };
    let point = annotation.to_visual(point);
    let color = chrome::color();
    draw::circle(painter, point, 6.5, 1.0, Color::WHITE, color, 2.0);
}
