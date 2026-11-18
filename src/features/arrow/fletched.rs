//! 【箭头】【燕尾轮廓】构造沿平滑中心线展开的单向或双向实心路径

use tiny_skia::{Path, PathBuilder};

use super::{
    ArrowStyle,
    metrics::{HeadMetrics, offset},
};
use crate::features::line::{
    Point,
    polyline::{Polyline, normal, normalized, sub},
};
use crate::renderer::paths::KAPPA;

/// 【箭头】【采样法线】返回 points 在 index 处的单位法线，两端使用相邻线段，中间使用中心差分
fn normal_at(points: &[Point], index: usize) -> Point {
    let before = points[index.saturating_sub(1)];
    let after = points[(index + 1).min(points.len() - 1)];
    normal(normalized(sub(after, before)))
}

/// 【箭头】【圆头尾帽】按 start、tangent、normal、left、right 与 half 向 builder 添加圆滑尾部，无返回值
fn tail_cap(
    builder: &mut PathBuilder,
    start: Point,
    tangent: Point,
    normal: Point,
    left: Point,
    right: Point,
    half: f32,
) {
    let back = (-tangent.0 * half, -tangent.1 * half);
    let middle = (start.0 + back.0, start.1 + back.1);
    let k = KAPPA * half;
    builder.cubic_to(
        left.0 - tangent.0 * k,
        left.1 - tangent.1 * k,
        middle.0 - normal.0 * k,
        middle.1 - normal.1 * k,
        middle.0,
        middle.1,
    );
    builder.cubic_to(
        middle.0 + normal.0 * k,
        middle.1 + normal.1 * k,
        right.0 - tangent.0 * k,
        right.1 - tangent.1 * k,
        right.0,
        right.1,
    );
}

/// 【箭头】【燕尾路径】以 line、metrics 和 style 生成闭合实心轮廓；返回路径或空值，不改变中心线
pub(super) fn path(line: &Polyline, metrics: HeadMetrics, style: ArrowStyle) -> Option<Path> {
    let bidirectional = style.bidirectional();
    let start_distance = if bidirectional { metrics.length } else { 0.0 };
    let shaft = line.section(start_distance, line.length - metrics.length);
    if shaft.len() < 2 {
        return None;
    }
    let start = line.points[0];
    let end = *line.points.last()?;
    let start_normal = normal(line.tangent(0.0));
    let end_normal = normal(line.tangent(line.length));
    let start_base = shaft[0];
    let end_base = *shaft.last()?;
    let half = metrics.body_half_width;
    let mut builder = PathBuilder::new();
    if bidirectional {
        builder.move_to(start.0, start.1);
        add(
            &mut builder,
            offset(start_base, start_normal, -metrics.half_width),
        );
        add(&mut builder, offset(start_base, start_normal, -half));
        for index in 1..shaft.len() {
            add(
                &mut builder,
                offset(shaft[index], normal_at(&shaft, index), -half),
            );
        }
        add(
            &mut builder,
            offset(end_base, end_normal, -metrics.half_width),
        );
        add(&mut builder, end);
        add(
            &mut builder,
            offset(end_base, end_normal, metrics.half_width),
        );
        add(&mut builder, offset(end_base, end_normal, half));
        for index in (0..shaft.len() - 1).rev() {
            add(
                &mut builder,
                offset(shaft[index], normal_at(&shaft, index), half),
            );
        }
        add(
            &mut builder,
            offset(start_base, start_normal, metrics.half_width),
        );
    } else {
        // 1. 【箭头】【尾部起点】尖尾沿原版采样轮廓直接展开，圆尾从完整半宽开始
        let tail_normal = normal_at(&shaft, 0);
        let tail_right = offset(shaft[0], tail_normal, half);
        let pointed = style.tail == super::ArrowTail::Pointed;
        let first = if pointed { start } else { tail_right };
        builder.move_to(first.0, first.1);
        for index in 1..shaft.len() {
            add(
                &mut builder,
                offset(shaft[index], normal_at(&shaft, index), half),
            );
        }
        add(
            &mut builder,
            offset(end_base, end_normal, metrics.half_width),
        );
        add(&mut builder, end);
        add(
            &mut builder,
            offset(end_base, end_normal, -metrics.half_width),
        );
        for index in (1..shaft.len()).rev() {
            add(
                &mut builder,
                offset(shaft[index], normal_at(&shaft, index), -half),
            );
        }
        // 2. 【箭头】【连续闭合】尖尾、主体和头部共用一条轮廓，避免拼接反向路径造成透明孔洞
        if !pointed {
            let tail_left = offset(shaft[0], tail_normal, -half);
            add(&mut builder, tail_left);
            tail_cap(
                &mut builder,
                start,
                line.tangent(0.0),
                tail_normal,
                tail_left,
                tail_right,
                half,
            );
        }
    }
    builder.close();
    builder.finish()
}

/// 【箭头】【轮廓顶点】将 point 追加到 builder；无返回值
fn add(builder: &mut PathBuilder, point: Point) {
    builder.line_to(point.0, point.1);
}
