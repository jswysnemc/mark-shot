//! 【尺子】【长度绘制】正文、刻度数字和选中把手共享完整窗口布局

use super::geometry::{rect_center, rotated};
use super::ticks::ticks_for_step;
use super::{Measurement, Point};
use crate::renderer::{Painter, paths::rounded_rect_path, text::HAlign};
use tiny_skia::{Color, LineCap, LineJoin, PathBuilder, Stroke, Transform};

/// 【尺子】【路径旋转】按center和degrees返回顺时针逻辑坐标变换
fn rotation_transform(center: Point, degrees: f32) -> Transform {
    let (sin, cos) = degrees.to_radians().sin_cos();
    Transform::from_row(
        cos,
        sin,
        -sin,
        cos,
        center.0 - cos * center.0 + sin * center.1,
        center.1 - sin * center.0 - cos * center.1,
    )
}

/// 【尺子】【刻度线段】将start、end绕center旋转rotation后加入path，无返回值
fn segment(path: &mut PathBuilder, start: Point, end: Point, center: Point, rotation: f32) {
    let start = rotated(start, center, rotation);
    let end = rotated(end, center, rotation);
    path.move_to(start.0, start.1);
    path.line_to(end.0, end.1);
}

/// 【尺子】【长度正文】按measurement、width和color在painter绘制完整条带、刻度及数字，无返回值
pub fn draw(painter: &mut Painter<'_>, measurement: Measurement, width: f32, color: Color) {
    if !measurement.measures() || color.alpha() <= 0.0 {
        return;
    }
    let Some(layout) = crate::renderer::ruler_chrome::layout(painter, measurement, width) else {
        return;
    };
    let bounds = layout.paint_rect;
    let center = rect_center(bounds);
    let unit = layout.unit;
    let rotation = layout.rotation;
    let mut fill = color;
    fill.apply_opacity(0.1);
    let pen = Stroke {
        width: layout.window.stroke(2.0 * unit, 1.0, None),
        line_cap: LineCap::Square,
        line_join: LineJoin::Bevel,
        ..Stroke::default()
    };
    // 1. 【尺子】【长度正文】正文保留实际尺寸，平直尺的命中框不会扩大测量正文
    if let Some(path) = rounded_rect_path(&bounds, 3.0 * unit, true, true, true, true)
        .and_then(|path| path.transform(rotation_transform(center, rotation)))
    {
        painter.fill_path(&path, fill);
        painter.stroke_path(&path, color, &pen);
    }
    let x_ticks = ticks_for_step(
        bounds.width() * measurement.pixel_scale,
        bounds.width(),
        layout.step_x,
    );
    let y_ticks = ticks_for_step(
        bounds.height() * measurement.pixel_scale,
        bounds.height(),
        layout.step_y,
    );
    let mut path = PathBuilder::new();
    // 2. 【尺子】【长度正文】四边刻度使用原版八段初始间隔与字体实际宽高
    for tick in &x_ticks {
        let x = bounds.left() + tick.position;
        let length = if tick.major { 13.0 } else { 6.0 } * unit;
        segment(
            &mut path,
            (x, bounds.top()),
            (x, bounds.top() - length),
            center,
            rotation,
        );
        segment(
            &mut path,
            (x, bounds.bottom()),
            (x, bounds.bottom() + length),
            center,
            rotation,
        );
    }
    for tick in &y_ticks {
        let y = bounds.top() + tick.position;
        let length = if tick.major { 13.0 } else { 6.0 } * unit;
        segment(
            &mut path,
            (bounds.left(), y),
            (bounds.left() - length, y),
            center,
            rotation,
        );
        segment(
            &mut path,
            (bounds.right(), y),
            (bounds.right() + length, y),
            center,
            rotation,
        );
    }
    if let Some(path) = path.finish() {
        painter.stroke_path(
            &path,
            color,
            &Stroke {
                width: layout.window.stroke(unit, 1.0, None),
                ..pen
            },
        );
    }
    // 3. 【尺子】【长度正文】单个数字只沿刻度方向移回允许范围，另一轴位置由完整条带决定
    for (ticks, horizontal) in [(&x_ticks, true), (&y_ticks, false)] {
        for tick in ticks.iter().filter(|tick| tick.major) {
            let position = if horizontal {
                bounds.left()
            } else {
                bounds.top()
            } + tick.position;
            let text = tick.pixel.to_string();
            let Some(rect) = layout.number(position, &text, horizontal) else {
                continue;
            };
            crate::renderer::ruler_text::halo_number(
                painter,
                rect,
                &text,
                color,
                rotation,
                center,
                crate::renderer::ruler_text::Style {
                    pixels: layout.font_pixels,
                    unit,
                    window: layout.window,
                    align: if horizontal {
                        HAlign::Center
                    } else if layout.right {
                        HAlign::Left
                    } else {
                        HAlign::Right
                    },
                },
            );
        }
    }
}
