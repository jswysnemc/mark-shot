//! 【线条】【描边轮廓】将实线、虚线和点线转为单次填充的闭合轮廓

use tiny_skia::{Color, Path};

use super::{LineSkeleton, LineStyle, clamped_width, geometry};
use crate::renderer::Painter;

/// 【线条】【描边轮廓】按 skeleton、style、width 生成闭合轮廓；退化骨架返回空值，描边细分比例随导出同步
pub fn outline(skeleton: &LineSkeleton, style: LineStyle, width: f32) -> Option<Path> {
    if !skeleton.valid() || geometry::sampled(skeleton)?.length < skeleton.style_scale {
        return None;
    }
    let width = clamped_width(width, skeleton.style_scale);
    stroke_outline(
        &geometry::path(skeleton)?,
        style,
        width,
        skeleton.style_scale,
    )
}

/// 【线条】【线型轮廓】把 path 按 style、width、scale 转为闭合描边；先切分虚线，非法比例返回空值
pub fn stroke_outline(path: &Path, style: LineStyle, width: f32, scale: f32) -> Option<Path> {
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    let mut stroke = style.stroke(clamped_width(width, scale));
    let resolution = 1.0 / scale;
    // 1. 【线条】【线型轮廓】轮廓接口不会读取 dash 参数，必须先生成各段开放路径
    match stroke.dash.take() {
        Some(dash) => path.dash(&dash, resolution)?.stroke(&stroke, resolution),
        None => path.stroke(&stroke, resolution),
    }
}

/// 【线条】【正文绘制】将 skeleton 以 style、width、color 绘入 painter；分块只合成一次，不叠加半透明笔色
pub fn draw(
    painter: &mut Painter<'_>,
    skeleton: &LineSkeleton,
    style: LineStyle,
    width: f32,
    color: Color,
) {
    if let Some(path) = outline(skeleton, style, width) {
        crate::renderer::fixed_path::fill(painter, &path, color);
    }
}
