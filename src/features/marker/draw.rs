//! 【形状标记】【共用绘制】工具图标、编辑预览与最终导出使用同一形状路径

use tiny_skia::{Color, LineCap, LineJoin, Rect, Stroke};

use super::path_for_shape;
use crate::renderer::Painter;
use crate::types::MarkerShape;

/// 【形状标记】【标注绘制】将 shape 绘入 painter 的 rect；color、width、filled 控制颜色、描边与填充，无返回值
pub fn draw_marker(
    painter: &mut Painter<'_>,
    shape: MarkerShape,
    rect: Rect,
    color: Color,
    width: f32,
    filled: bool,
) {
    let Some(path) = path_for_shape(shape, rect) else {
        return;
    };
    if filled {
        painter.fill_path(&path, color);
    }
    let stroke = Stroke {
        width: painter.window_units().stroke(width, 1.5, None),
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        ..Stroke::default()
    };
    painter.stroke_path(&path, color, &stroke);
}

/// 【形状标记】【图标绘制】将 shape 与 filled 状态绘入 painter 的 rect；color 指定主题颜色，无返回值
pub fn draw_icon(
    painter: &mut Painter<'_>,
    shape: MarkerShape,
    rect: Rect,
    color: Color,
    filled: bool,
) {
    draw_marker(painter, shape, rect, color, 1.5, filled);
}
