//! 【箭头】【组合路径】箭身使用独立线型，所有头部保持连续，并共用一次填充

use tiny_skia::{Path, PathBuilder};

use super::{
    ArrowStyle, fletched,
    metrics::{self, HeadMetrics, triangle},
};
use crate::features::line::{
    LineSkeleton, LineStyle, Point, clamped_width, geometry,
    polyline::{Polyline, distance, normalized, sub},
};

/// 【箭头】【组合轮廓】根据 skeleton、style、line_style、width 生成完整闭合路径；退化输入返回空值
pub fn outline(
    skeleton: &LineSkeleton,
    style: ArrowStyle,
    line_style: LineStyle,
    width: f32,
) -> Option<Path> {
    let line = geometry::sampled(skeleton)?;
    if line.length < skeleton.style_scale {
        return None;
    }
    let width = clamped_width(width, skeleton.style_scale);
    let metrics = metrics::metrics(line.length, width, skeleton.style_scale, style);
    if !style.open() && line_style == LineStyle::Solid {
        return fletched::path(&line, metrics, style);
    }
    let mut builder = PathBuilder::new();
    let center = if style.open() {
        geometry::path(skeleton)?
    } else {
        let points = line.section(
            if style.bidirectional() {
                metrics.length
            } else {
                0.0
            },
            line.length - metrics.length,
        );
        polyline_path(&points)?
    };
    builder.push_path(&crate::features::line::draw::stroke_outline(
        &center,
        line_style,
        width,
        skeleton.style_scale,
    )?);
    let mut heads = PathBuilder::new();
    for vertices in head_triangles(&line, metrics, style) {
        // 1. 【箭头】【填充绕向】实心头部与描边外轮廓同向，非零填充不会抵消接缝
        let vertices = if style.open() {
            vertices
        } else {
            [vertices[2], vertices[1], vertices[0]]
        };
        heads.move_to(vertices[0].0, vertices[0].1);
        heads.line_to(vertices[1].0, vertices[1].1);
        heads.line_to(vertices[2].0, vertices[2].1);
        if !style.open() {
            heads.close();
        }
    }
    let heads = heads.finish()?;
    // 2. 【箭头】【连续头部】头部始终使用实线，不把箭身的虚线或点线传入头部
    if style.open() {
        builder
            .push_path(&heads.stroke(&LineStyle::Solid.stroke(width), 1.0 / skeleton.style_scale)?);
    } else {
        builder.push_path(&heads);
    }
    builder.finish()
}

/// 【箭头】【头部集合】从 line、metrics 和 style 返回一组或两组头部顶点，基点使用实际弧长
fn head_triangles(line: &Polyline, metrics: HeadMetrics, style: ArrowStyle) -> Vec<[Point; 3]> {
    let end = *line.points.last().expect("a polyline has endpoints");
    let end_direction = line.tangent(line.length);
    let end_base = if style.open() {
        metrics::offset(end, end_direction, -metrics.length)
    } else {
        line.at(line.length - metrics.length)
    };
    let mut heads = vec![triangle(end, end_base, end_direction, metrics.half_width)];
    if style.bidirectional() {
        let start = line.points[0];
        let direction = line.tangent(0.0);
        let base = if style.open() {
            metrics::offset(start, direction, metrics.length)
        } else {
            line.at(metrics.length)
        };
        heads.push(triangle(
            start,
            base,
            (-direction.0, -direction.1),
            metrics.half_width,
        ));
    }
    heads
}

/// 【箭头】【箭身路径】用 points 构造开放折线，返回路径或空值
fn polyline_path(points: &[Point]) -> Option<Path> {
    let first = points.first()?;
    let mut builder = PathBuilder::new();
    builder.move_to(first.0, first.1);
    for point in &points[1..] {
        builder.line_to(point.0, point.1);
    }
    builder.finish()
}

/// 【箭头】【正文命中】判断 point 是否位于 skeleton 或其头部 tolerance 范围内；不把虚线空隙视为不可选择
pub fn hit_test(
    skeleton: &LineSkeleton,
    style: ArrowStyle,
    width: f32,
    point: Point,
    tolerance: f32,
) -> bool {
    if !point.0.is_finite() || !point.1.is_finite() || !tolerance.is_finite() || tolerance < 0.0 {
        return false;
    }
    let width = clamped_width(width, skeleton.style_scale);
    if geometry::insertion_at(skeleton, point, tolerance + width * 0.5).is_some() {
        return true;
    }
    let Some(line) = geometry::sampled(skeleton) else {
        return false;
    };
    let metrics = metrics::metrics(line.length, width, skeleton.style_scale, style);
    head_triangles(&line, metrics, style)
        .into_iter()
        .any(|vertices| {
            let edges = [[vertices[0], vertices[1]], [vertices[1], vertices[2]]];
            edges.into_iter().any(|edge| {
                segment_distance(point, edge[0], edge[1])
                    <= tolerance + if style.open() { width * 0.5 } else { 0.0 }
            }) || (!style.open() && inside_triangle(point, vertices))
        })
}

/// 【箭头】【线段距离】返回 point 到 start、end 线段的最短距离，退化线段按点处理
fn segment_distance(point: Point, start: Point, end: Point) -> f32 {
    let direction = normalized(sub(end, start));
    let length = distance(start, end);
    let offset = sub(point, start);
    let projected = (offset.0 * direction.0 + offset.1 * direction.1).clamp(0.0, length);
    distance(point, metrics::offset(start, direction, projected))
}

/// 【箭头】【实心头部命中】返回 point 是否位于 triangle 内，不要求顶点为固定绕向
fn inside_triangle(point: Point, triangle: [Point; 3]) -> bool {
    let first = sub(triangle[1], triangle[0]);
    let second = sub(triangle[2], triangle[0]);
    if (first.0 * second.1 - first.1 * second.0).abs() <= 0.00001 {
        return false;
    }
    let mut positive = false;
    let mut negative = false;
    for index in 0..3 {
        let start = triangle[index];
        let end = triangle[(index + 1) % 3];
        let value =
            (end.0 - start.0) * (point.1 - start.1) - (end.1 - start.1) * (point.0 - start.0);
        positive |= value > 0.0;
        negative |= value < 0.0;
    }
    !positive || !negative
}
