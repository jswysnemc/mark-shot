//! 【线条】【平滑几何】复用原版单控制点二次曲线及多控制点连续二次曲线规则

use tiny_skia::{Path, PathBuilder, Rect};

use super::{
    LineSkeleton, Point,
    polyline::{Polyline, distance, lerp},
};

const MAX_SAMPLES: usize = 8192;

#[derive(Clone, Copy)]
enum Segment {
    Line(Point),
    Quad(Point, Point),
}

#[derive(Clone, Copy, Debug)]
pub struct Insertion {
    pub index: usize,
    pub point: Point,
    pub distance: f32,
}

/// 【线条】【平滑分段】将 skeleton 转为绘制顺序中的线段或二次曲线；非法骨架返回空值
fn segments(skeleton: &LineSkeleton) -> Option<Vec<Segment>> {
    if !skeleton.valid() {
        return None;
    }
    let mut route = Vec::with_capacity(skeleton.controls.len() + 2);
    route.push(skeleton.start);
    route.extend(skeleton.controls.iter().copied());
    route.push(skeleton.end);
    let result = match route.len() {
        2 => vec![Segment::Line(skeleton.end)],
        3 => vec![Segment::Quad(route[1], skeleton.end)],
        _ => {
            let mut result = vec![Segment::Line(lerp(route[0], route[1], 0.5))];
            // 1. 【线条】【平滑分段】相邻控制点共享中点，局部曲率连续且保持末端坐标精确
            for index in 1..route.len() - 1 {
                result.push(Segment::Quad(
                    route[index],
                    lerp(route[index], route[index + 1], 0.5),
                ));
            }
            result.push(Segment::Line(skeleton.end));
            result
        }
    };
    Some(result)
}

/// 【线条】【骨架路径】从 skeleton 创建可直接描边的平滑路径；返回路径或空值，不修改定点
pub fn path(skeleton: &LineSkeleton) -> Option<Path> {
    let segments = segments(skeleton)?;
    let mut builder = PathBuilder::new();
    builder.move_to(skeleton.start.0, skeleton.start.1);
    for segment in segments {
        match segment {
            Segment::Line(end) => builder.line_to(end.0, end.1),
            Segment::Quad(control, end) => builder.quad_to(control.0, control.1, end.0, end.1),
        }
    }
    builder.finish()
}

/// 【线条】【曲线细分】将 start、control、end 按 tolerance 细分到 points；depth 限制递归，最多八千一百九十二点，无返回值
fn flatten(
    start: Point,
    control: Point,
    end: Point,
    tolerance: f32,
    depth: u8,
    points: &mut Vec<Point>,
) {
    if points.len() >= MAX_SAMPLES {
        return;
    }
    let midpoint = lerp(start, end, 0.5);
    if depth >= 10 || distance(control, midpoint) <= tolerance {
        points.push(end);
        return;
    }
    let first = lerp(start, control, 0.5);
    let second = lerp(control, end, 0.5);
    let middle = lerp(first, second, 0.5);
    flatten(start, first, middle, tolerance, depth + 1, points);
    flatten(middle, second, end, tolerance, depth + 1, points);
}

/// 【线条】【有限中心线】将 skeleton 细分并均匀重采样；返回至多五百一十二段中心线，误差随原图比例同步
pub fn sampled(skeleton: &LineSkeleton) -> Option<Polyline> {
    let segments = segments(skeleton)?;
    let tolerance = 0.25 * skeleton.style_scale;
    let mut points = vec![skeleton.start];
    let mut start = skeleton.start;
    for segment in segments {
        match segment {
            Segment::Line(end) => {
                if points.len() < MAX_SAMPLES {
                    points.push(end);
                }
                start = end;
            }
            Segment::Quad(control, end) => {
                flatten(start, control, end, tolerance, 0, &mut points);
                start = end;
            }
        }
    }
    // 1. 【线条】【有限中心线】容量限制不能截断箭头末端，始终保留精确终点
    if points.last().copied() != Some(skeleton.end) {
        if points.len() >= MAX_SAMPLES {
            points.pop();
        }
        points.push(skeleton.end);
    }
    let line = Polyline::new(points)?;
    if skeleton.controls.is_empty() {
        Some(line)
    } else {
        Some(line.resampled((skeleton.controls.len() * 4).clamp(64, 512)))
    }
}

/// 【线条】【插入定位】寻找 skeleton 距 point 不超过 tolerance 的骨架插入位置；返回原版点索引和投影坐标
pub fn insertion_at(skeleton: &LineSkeleton, point: Point, tolerance: f32) -> Option<Insertion> {
    if !point.0.is_finite() || !point.1.is_finite() || !tolerance.is_finite() || tolerance < 0.0 {
        return None;
    }
    let line = sampled(skeleton)?;
    let (point, distance, progress) = line.nearest(point);
    if distance > tolerance {
        return None;
    }
    let segment_count = skeleton.controls.len() + 1;
    let segment_index = (progress * segment_count as f32).floor() as usize;
    let index = segment_index.min(segment_count - 1) + 2;
    Some(Insertion {
        index,
        point,
        distance,
    })
}

/// 【线条】【定点边界】返回 skeleton 全部定点的最小有效外框，供退化草稿和界面操作使用
pub fn anchor_bounds(skeleton: &LineSkeleton) -> Option<Rect> {
    if !skeleton.valid() {
        return None;
    }
    let min_x = skeleton
        .points()
        .map(|point| point.0)
        .fold(f32::INFINITY, f32::min);
    let max_x = skeleton
        .points()
        .map(|point| point.0)
        .fold(f32::NEG_INFINITY, f32::max);
    let min_y = skeleton
        .points()
        .map(|point| point.1)
        .fold(f32::INFINITY, f32::min);
    let max_y = skeleton
        .points()
        .map(|point| point.1)
        .fold(f32::NEG_INFINITY, f32::max);
    Rect::from_ltrb(min_x - 0.5, min_y - 0.5, max_x + 0.5, max_y + 0.5)
}

/// 【线条】【绘图吸附】把 start 到 end 的方向吸附到最近四十五度并保持长度；返回吸附后的终点
pub fn snapped_end(start: Point, end: Point) -> Point {
    let delta = (end.0 - start.0, end.1 - start.1);
    let length = delta.0.hypot(delta.1);
    let angle = (delta.1.atan2(delta.0) / std::f32::consts::FRAC_PI_4).round()
        * std::f32::consts::FRAC_PI_4;
    (
        start.0 + length * angle.cos(),
        start.1 + length * angle.sin(),
    )
}
