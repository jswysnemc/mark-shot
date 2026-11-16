//! 【箭头】【正文绘制】在固定像素块中绘制组合轮廓，不重复叠加半透明箭头

use tiny_skia::{Color, Transform};

use super::{ArrowStyle, path};
use crate::features::line::{LineSkeleton, LineStyle};
use crate::renderer::Painter;

/// 【箭头】【正文绘制】把 skeleton 按 style、line_style、width、color 与 shadow_color 绘入 painter；无返回值
pub fn draw(
    painter: &mut Painter<'_>,
    skeleton: &LineSkeleton,
    style: ArrowStyle,
    line_style: LineStyle,
    width: f32,
    color: Color,
    shadow_color: Color,
) {
    let width = painter.window_units().stroke(width, 1.5, None);
    if let Some(path) = path::outline_rendered(
        skeleton,
        style,
        line_style,
        width,
        painter.window_units().canvas(1.0),
    ) {
        if shadow_color.alpha() > 0.0 {
            let [dx, dy] = crate::config::get().annotations.shadow.offset;
            if dx.is_finite()
                && dy.is_finite()
                && let Some(shadow) = path.clone().transform(Transform::from_translate(dx, dy))
            {
                crate::renderer::fixed_path::fill(painter, &shadow, shadow_color);
            }
        }
        crate::renderer::fixed_path::fill(painter, &path, color);
    }
}
