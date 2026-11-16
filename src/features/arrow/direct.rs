//! 【箭头】【直箭头头部】实心内凹箭头配合圆头箭身，头部缺口与箭杆相接

use tiny_skia::{Path, PathBuilder};

use super::{ArrowStyle, metrics::HeadMetrics};
use crate::features::line::{
    LineStyle, Point,
    polyline::{Polyline, normal},
};

/// 【箭头】【缺口比例】返回直箭头缺口到尖端的距离比例，非法配置回退到三分之二
fn notch_ratio() -> f32 {
    let value = crate::config::get().annotations.arrow.head_notch;
    if value.is_finite() {
        value.clamp(0.1, 0.95)
    } else {
        0.667
    }
}

/// 【箭头】【直箭头路径】把圆头箭身和内凹实心头部合成闭合路径；退化输入返回空值
pub(super) fn path(
    line: &Polyline,
    metrics: HeadMetrics,
    style: ArrowStyle,
    line_style: LineStyle,
    width: f32,
    scale: f32,
) -> Option<Path> {
    let notch = metrics.length * notch_ratio();
    let start = if style.bidirectional() { notch } else { 0.0 };
    let center = line.section(start, line.length - notch);
    let center = super::path::polyline_path(&center)?;
    let mut builder = PathBuilder::new();
    builder.push_path(&crate::features::line::draw::stroke_outline_rendered(
        &center, line_style, width, scale,
    )?);
    let end = *line.points.last()?;
    dart(
        &mut builder,
        end,
        line.tangent(line.length),
        line.at(line.length - metrics.length),
        line.at(line.length - notch),
        metrics.half_width,
    );
    if style.bidirectional() {
        let start = line.points[0];
        let direction = line.tangent(0.0);
        dart(
            &mut builder,
            start,
            (-direction.0, -direction.1),
            line.at(metrics.length),
            line.at(notch),
            metrics.half_width,
        );
    }
    builder.finish()
}

/// 【箭头】【内凹头部】按 tip、direction、barbs、notch 构造同一绕向的实心箭头，窄边与箭身相接
fn dart(
    builder: &mut PathBuilder,
    tip: Point,
    direction: Point,
    barbs: Point,
    notch: Point,
    half_width: f32,
) {
    let normal = normal(direction);
    builder.move_to(
        barbs.0 - normal.0 * half_width,
        barbs.1 - normal.1 * half_width,
    );
    builder.line_to(tip.0, tip.1);
    builder.line_to(
        barbs.0 + normal.0 * half_width,
        barbs.1 + normal.1 * half_width,
    );
    builder.line_to(notch.0, notch.1);
    builder.close();
}
