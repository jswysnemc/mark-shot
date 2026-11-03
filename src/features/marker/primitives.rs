//! 【形状标记】【基础路径】构造单位坐标中的多边形、圆形和圆角线段

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use tiny_skia::{LineCap, LineJoin, Path, PathBuilder, Rect, Stroke};

/// 【形状标记】【多边形】将 points 按顺序连接并闭合；返回单位路径，少于三点返回空值
pub(super) fn polygon(points: &[(f32, f32)]) -> Option<Path> {
    let &(x, y) = points.first().filter(|_| points.len() >= 3)?;
    let mut builder = PathBuilder::new();
    builder.move_to(x, y);
    for &(x, y) in &points[1..] {
        builder.line_to(x, y);
    }
    builder.close();
    builder.finish()
}

/// 【形状标记】【正多边形】根据 sides、radius 与起始角 angle 构造单位路径；返回闭合轮廓
pub(super) fn regular_polygon(sides: usize, radius: f32, angle: f32) -> Option<Path> {
    let points: Vec<_> = (0..sides)
        .map(|index| {
            let angle = angle + index as f32 * TAU / sides as f32;
            (0.5 + radius * angle.cos(), 0.5 + radius * angle.sin())
        })
        .collect();
    polygon(&points)
}

/// 【形状标记】【星形】交替使用 outer、inner 半径构造尖角数量 points 的星形；返回单位路径
pub(super) fn star(points: usize, outer: f32, inner: f32) -> Option<Path> {
    let vertices: Vec<_> = (0..points * 2)
        .map(|index| {
            let radius = if index % 2 == 0 { outer } else { inner };
            let angle = -FRAC_PI_2 + index as f32 * PI / points as f32;
            (0.5 + radius * angle.cos(), 0.5 + radius * angle.sin())
        })
        .collect();
    polygon(&vertices)
}

/// 【形状标记】【圆形】以 x、y 为圆心、radius 为半径构造单位路径；返回闭合圆形
pub(super) fn circle(x: f32, y: f32, radius: f32) -> Option<Path> {
    PathBuilder::from_circle(x, y, radius)
}

/// 【形状标记】【圆角矩形】在 bounds 中使用 radius 构造圆角单位路径；返回闭合轮廓
pub(super) fn rounded_rectangle(bounds: Rect, radius: f32) -> Option<Path> {
    crate::renderer::rounded_rect_path(&bounds, radius, true, true, true, true)
}

/// 【形状标记】【圆角线段】将 points 中心线按 width 加粗为填充轮廓；返回单位路径
pub(super) fn round_line(points: &[(f32, f32)], width: f32) -> Option<Path> {
    let &(x, y) = points.first().filter(|_| points.len() >= 2)?;
    let mut builder = PathBuilder::new();
    builder.move_to(x, y);
    for &(x, y) in &points[1..] {
        builder.line_to(x, y);
    }
    builder.finish()?.stroke(
        &Stroke {
            width,
            line_cap: LineCap::Round,
            line_join: LineJoin::Round,
            ..Stroke::default()
        },
        1.0,
    )
}
