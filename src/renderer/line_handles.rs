//! 【线条】【骨架控制框】复用统一外框，绘制端点、外部控制点、旋转及当前定点提示

use tiny_skia::{Color, PathBuilder, Stroke};

use super::Painter;
use crate::features::line::drag;
use crate::types::Annotation;

/// 【线条】【控制框绘制】将 annotation 的外框和定点绘入 painter；不修改正文或历史，无返回值
pub(crate) fn draw(painter: &mut Painter<'_>, annotation: &Annotation) {
    let Some(line) = annotation.shape.line() else {
        return;
    };
    let Some(frame) = drag::frame(annotation.bbox) else {
        return;
    };
    super::annotations::draw_exact_frame_handles(painter, &frame);
    let color = crate::config::get().annotations.handles.color.get().color();
    let rotation = drag::rotation_point(line, annotation.bbox);
    let mut connector = PathBuilder::new();
    connector.move_to(rotation.0, frame.top());
    connector.line_to(rotation.0, rotation.1);
    if let Some(path) = connector.finish() {
        painter.stroke_path(
            &path,
            color,
            &Stroke {
                width: 1.0,
                ..Stroke::default()
            },
        );
    }
    painter.fill_circle(
        rotation.0,
        rotation.1,
        5.0,
        Color::from_rgba8(8, 13, 19, 255),
    );
    painter.fill_circle(rotation.0, rotation.1, 3.0, color);
    for (index, point) in line.points().enumerate() {
        let accent = crate::config::get().theme.accent.color();
        painter.fill_circle(
            point.0,
            point.1,
            6.0,
            if index < 2 { color } else { accent },
        );
        painter.fill_circle(
            point.0,
            point.1,
            4.0,
            if index < 2 { accent } else { color },
        );
    }
}

/// 【线条】【当前定点提示】将 annotation 中 index 定点强调绘入 painter，告知删除目标；越界时不绘制，无返回值
pub(crate) fn highlight(painter: &mut Painter<'_>, annotation: &Annotation, index: usize) {
    let Some(point) = annotation.shape.line().and_then(|line| line.point(index)) else {
        return;
    };
    let color = crate::config::get().annotations.handles.color.get().color();
    let accent = crate::config::get().theme.accent.color();
    painter.fill_circle(point.0, point.1, 7.5, accent);
    painter.fill_circle(point.0, point.1, 5.5, color);
}
