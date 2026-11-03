//! 【箭头】【正文绘制】在固定像素块中绘制组合轮廓，不重复叠加半透明箭头

use tiny_skia::Color;

use super::{ArrowStyle, path};
use crate::features::line::{LineSkeleton, LineStyle};
use crate::renderer::Painter;

/// 【箭头】【正文绘制】把 skeleton 按 style、line_style、width、color 绘入 painter；无返回值
pub fn draw(
    painter: &mut Painter<'_>,
    skeleton: &LineSkeleton,
    style: ArrowStyle,
    line_style: LineStyle,
    width: f32,
    color: Color,
) {
    if let Some(path) = path::outline(skeleton, style, line_style, width) {
        crate::renderer::fixed_path::fill(painter, &path, color);
    }
}
