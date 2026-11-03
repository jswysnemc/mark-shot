//! 【尺子】【长度绘制】绘制测量框、双侧刻度和随框旋转的数字

use tiny_skia::{Color, LineCap, LineJoin, PathBuilder, Rect, Stroke, Transform};

use super::geometry::{contains, rect_center, rotated};
use super::labels::number_rect;
use super::ticks::ticks;
use super::{Measurement, Point, RulerGeometry};
use crate::renderer::Painter;
use crate::renderer::paths::rounded_rect_path;

/// 【尺子】【路径旋转】返回围绕 center 顺时针旋转 degrees 的逻辑坐标变换
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

/// 【尺子】【刻度线段】把 start、end 绕 center 旋转 rotation 后加入 path；无返回值
fn segment(path: &mut PathBuilder, start: Point, end: Point, center: Point, rotation: f32) {
    let start = rotated(start, center, rotation);
    let end = rotated(end, center, rotation);
    path.move_to(start.0, start.1);
    path.line_to(end.0, end.1);
}

/// 【尺子】【长度正文】将 measurement 按 width 与 color 绘入 painter；数字遵循显式绘图边界，不受局部重绘缓冲大小影响
pub fn draw(painter: &mut Painter<'_>, measurement: Measurement, width: f32, color: Color) {
    let RulerGeometry::Length {
        start,
        end,
        rotation,
    } = measurement.geometry
    else {
        return;
    };
    if !measurement.measures() || color.alpha() <= 0.0 {
        return;
    }
    let Some(bounds) = measurement.unrotated_bounds() else {
        return;
    };
    let center = rect_center(bounds);
    let unit = measurement.unit(width);
    let mut fill = color;
    fill.apply_opacity(0.1);
    let pen = Stroke {
        width: (2.0 * unit).max(measurement.style_scale),
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        ..Stroke::default()
    };
    // 1. 【尺子】【长度正文】测量框与刻度共用中心旋转，填充保留截图内容
    if let Some(path) = rounded_rect_path(&bounds, 3.0 * unit, true, true, true, true)
        .and_then(|path| path.transform(rotation_transform(center, rotation)))
    {
        painter.fill_path(&path, fill);
        painter.stroke_path(&path, color, &pen);
    }
    let font_size = 13.0 * unit;
    let source_width = (end.0 - start.0).abs() * measurement.pixel_scale;
    let source_height = (end.1 - start.1).abs() * measurement.pixel_scale;
    let longest = (source_width.round() as u64)
        .max(source_height.round() as u64)
        .to_string();
    let text_width = painter
        .measure(&longest, font_size)
        .max(longest.len() as f32 * 7.0 * unit);
    let x_ticks = ticks(
        source_width,
        (end.0 - start.0).abs(),
        text_width + 6.0 * unit,
    );
    let y_ticks = ticks(source_height, (end.1 - start.1).abs(), 20.0 * unit);
    let allowed = painter.drawing_bounds();
    let above = rotated((center.0, bounds.top() - 23.0 * unit), center, rotation);
    let below = allowed.is_some_and(|bounds| !contains(bounds, above));
    let left = rotated(
        (bounds.left() - text_width / 2.0 - 14.0 * unit, center.1),
        center,
        rotation,
    );
    let right = allowed.is_some_and(|bounds| !contains(bounds, left));
    let mut path = PathBuilder::new();
    // 2. 【尺子】【长度正文】双侧刻度以原图像素编号，主刻度按可读数字间距自动选择
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
                width: (1.3 * unit).max(measurement.style_scale),
                ..pen
            },
        );
    }
    // 3. 【尺子】【长度正文】数字先约束到允许范围，再与刻度一起旋转，保留边缘可读性
    for tick in x_ticks.iter().filter(|tick| tick.major) {
        let x = bounds.left() + tick.position;
        let y = if below {
            bounds.bottom() + 14.0 * unit
        } else {
            bounds.top() - 30.0 * unit
        };
        let rect =
            Rect::from_xywh(x - text_width / 2.0, y, text_width.max(1.0), 16.0 * unit).unwrap();
        let anchor = rotated(
            (x, if below { bounds.bottom() } else { bounds.top() }),
            center,
            rotation,
        );
        let rect = if rotation.abs() < 0.001 {
            number_rect(rect, anchor, allowed)
        } else {
            rect
        };
        crate::renderer::ruler_text::halo_number(
            painter,
            rect,
            &tick.pixel.to_string(),
            font_size,
            color,
            rotation,
            center,
        );
    }
    for tick in y_ticks.iter().filter(|tick| tick.major) {
        let y = bounds.top() + tick.position;
        let x = if right {
            bounds.right() + 14.0 * unit
        } else {
            bounds.left() - 14.0 * unit - text_width
        };
        let rect = Rect::from_xywh(x, y - 8.0 * unit, text_width.max(1.0), 16.0 * unit).unwrap();
        let anchor = rotated(
            (if right { bounds.right() } else { bounds.left() }, y),
            center,
            rotation,
        );
        let rect = if rotation.abs() < 0.001 {
            number_rect(rect, anchor, allowed)
        } else {
            rect
        };
        crate::renderer::ruler_text::halo_number(
            painter,
            rect,
            &tick.pixel.to_string(),
            font_size,
            color,
            rotation,
            center,
        );
    }
}
