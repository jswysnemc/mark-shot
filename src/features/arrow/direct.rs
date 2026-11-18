//! 【箭头】【直箭头头部】实心内凹箭头配合圆头箭身，头部缺口与箭杆相接

use tiny_skia::{Path, PathBuilder};

use super::{ArrowStyle, metrics::HeadMetrics};
use crate::features::line::{
    LineStyle, Point,
    polyline::{Polyline, normal},
};

/// 【箭头】【缺口比例】无参数，返回直箭头缺口到尖端的距离比例，非法配置回退到三分之二
fn notch_ratio() -> f32 {
    let value = crate::config::get().annotations.arrow.head_notch;
    if value.is_finite() {
        value.clamp(0.1, 0.95)
    } else {
        0.667
    }
}

/// 【箭头】【直箭头路径】按 line、metrics、style、line_style、width 与 scale 合成圆头箭身和内凹头部，返回闭合路径或空值
pub(super) fn path(
    line: &Polyline,
    metrics: HeadMetrics,
    style: ArrowStyle,
    line_style: LineStyle,
    width: f32,
    scale: f32,
) -> Option<Path> {
    // 1. 【箭头】【箭身合成】按缺口位置裁切中线，保持线型和实际窗口笔宽
    let notch = metrics.length * notch_ratio();
    let start = if style.bidirectional() { notch } else { 0.0 };
    let center = line.section(start, line.length - notch);
    let center = super::path::polyline_path(&center)?;
    let mut builder = PathBuilder::new();
    let shaft = if style.tail == super::ArrowTail::Pointed && !style.bidirectional() {
        super::tail::shaft(line, line.length - notch, line_style, width, scale)?
    } else {
        crate::features::line::draw::stroke_outline_rendered(&center, line_style, width, scale)?
    };
    builder.push_path(&shaft);
    let end = *line.points.last()?;
    // 2. 【箭头】【头部合成】终点始终绘制头部，双向样式增加起点头部
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

/// 【箭头】【内凹头部】按 tip、direction、barbs、notch 与 half_width 向 builder 添加实心箭头，无返回值
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
