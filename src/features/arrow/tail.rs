//! 【箭头】【尖状尾部】沿实际骨架逐渐展开尾部，箭身线型与头部保持独立

use crate::features::line::{
    LineStyle,
    polyline::{Polyline, normal},
};
use tiny_skia::{Path, PathBuilder};

/// 【箭头】【尖尾箭身】按 line、end、line_style、width 和 scale 返回从尖端展开的箭身轮廓；退化骨架返回空值
pub(super) fn shaft(
    line: &Polyline,
    end: f32,
    line_style: LineStyle,
    width: f32,
    scale: f32,
) -> Option<Path> {
    let span = if line_style == LineStyle::Solid {
        crate::features::line::polyline::distance(line.points[0], line.points[1]).min(end)
    } else {
        (width * 3.0).min(end * 0.5)
    };
    if span <= 0.0 {
        return None;
    }
    let points = line.section(0.0, span);
    let mut sides = Vec::with_capacity(points.len());
    let mut distance = 0.0;
    for (index, &point) in points.iter().enumerate() {
        if index > 0 {
            distance += crate::features::line::polyline::distance(points[index - 1], point);
        }
        let half = width * 0.5 * (distance / span).min(1.0);
        let direction = normal(line.tangent(distance));
        sides.push((
            (point.0 + direction.0 * half, point.1 + direction.1 * half),
            (point.0 - direction.0 * half, point.1 - direction.1 * half),
        ));
    }
    let first = sides.first()?.1;
    let mut builder = PathBuilder::new();
    // 1. 【箭头】【尾部展开】尾端宽度为零，沿弧长增至箭身笔宽，所有子路径保持相同填充绕向
    builder.move_to(first.0, first.1);
    for &(_, right) in &sides[1..] {
        builder.line_to(right.0, right.1);
    }
    for &(left, _) in sides.iter().rev() {
        builder.line_to(left.0, left.1);
    }
    builder.close();
    if span >= end {
        return builder.finish();
    }
    let center = super::path::polyline_path(&line.section(span, end))?;
    builder.push_path(&crate::features::line::draw::stroke_outline_rendered(
        &center, line_style, width, scale,
    )?);
    builder.finish()
}
