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

/// 【箭头】【采样法线】返回 points 在 index 处的单位法线，两端使用相邻线段，中间使用中心差分
fn normal_at(points: &[Point], index: usize) -> Point {
    let before = points[index.saturating_sub(1)];
    let after = points[(index + 1).min(points.len() - 1)];
    normal(normalized(sub(after, before)))
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
    let mut builder = PathBuilder::new();
    builder.move_to(start.0, start.1);
    if bidirectional {
        add(
            &mut builder,
            offset(start_base, start_normal, -metrics.half_width),
        );
        add(
            &mut builder,
            offset(start_base, start_normal, -metrics.body_half_width),
        );
        for index in 1..shaft.len() {
            add(
                &mut builder,
                offset(
                    shaft[index],
                    normal_at(&shaft, index),
                    -metrics.body_half_width,
                ),
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
        add(
            &mut builder,
            offset(end_base, end_normal, metrics.body_half_width),
        );
        for index in (0..shaft.len() - 1).rev() {
            add(
                &mut builder,
                offset(
                    shaft[index],
                    normal_at(&shaft, index),
                    metrics.body_half_width,
                ),
            );
        }
        add(
            &mut builder,
            offset(start_base, start_normal, metrics.half_width),
        );
    } else {
        // 1. 【箭头】【燕尾路径】单向箭头从零宽尾部开始，曲线两侧使用各自采样法线
        for index in 1..shaft.len() {
            add(
                &mut builder,
                offset(
                    shaft[index],
                    normal_at(&shaft, index),
                    metrics.body_half_width,
                ),
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
                offset(
                    shaft[index],
                    normal_at(&shaft, index),
                    -metrics.body_half_width,
                ),
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
