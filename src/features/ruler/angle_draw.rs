//! 【尺子】【角度绘制】绘制两条测量边、较小夹角圆弧、扇形与定点

use tiny_skia::{Color, LineCap, LineJoin, PathBuilder, Stroke};

use super::labels::angle_arc;
use super::{Measurement, RulerGeometry};
use crate::renderer::Painter;

/// 【尺子】【角度正文】按 measurement 的已确认点、width 和 color 绘入 painter；未完成第二边时仍显示第一边，无返回值
pub fn draw(painter: &mut Painter<'_>, measurement: Measurement, width: f32, color: Color) {
    let RulerGeometry::Angle { points, count } = measurement.geometry else {
        return;
    };
    if count < 2 || color.alpha() <= 0.0 {
        return;
    }
    let unit = measurement.unit(width);
    let pen = Stroke {
        width: (2.0 * unit).max(measurement.style_scale as f32),
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        ..Stroke::default()
    };
    // 1. 【尺子】【角度正文】三点完整时绘制较小一侧圆弧，短边限定圆弧最大半径
    if let Some((radius, start, sweep)) = angle_arc(measurement, unit) {
        let steps = ((sweep.abs() * 24.0).ceil() as u32).clamp(2, 96);
        let mut arc = PathBuilder::new();
        let mut wedge = PathBuilder::new();
        wedge.move_to(points[0].0, points[0].1);
        for step in 0..=steps {
            let angle = start + sweep * step as f32 / steps as f32;
            let point = (
                points[0].0 + angle.cos() * radius,
                points[0].1 + angle.sin() * radius,
            );
            if step == 0 {
                arc.move_to(point.0, point.1);
            } else {
                arc.line_to(point.0, point.1);
            }
            wedge.line_to(point.0, point.1);
        }
        wedge.close();
        let mut fill = color;
        fill.apply_opacity(0.16);
        if let Some(path) = wedge.finish() {
            painter.fill_path(&path, fill);
        }
        if let Some(path) = arc.finish() {
            painter.stroke_path(
                &path,
                color,
                &Stroke {
                    width: (1.5 * unit).max(measurement.style_scale as f32),
                    ..pen.clone()
                },
            );
        }
    }
    // 2. 【尺子】【角度正文】两条边和三个定点使用同一样式单位
    let mut path = PathBuilder::new();
    for point in &points[1..usize::from(count.min(3))] {
        path.move_to(points[0].0, points[0].1);
        path.line_to(point.0, point.1);
    }
    if let Some(path) = path.finish() {
        painter.stroke_path(&path, color, &pen);
    }
    painter.fill_circle(points[0].0, points[0].1, 4.0 * unit, color);
    for point in &points[1..usize::from(count.min(3))] {
        painter.fill_circle(point.0, point.1, 3.0 * unit, color);
    }
}
