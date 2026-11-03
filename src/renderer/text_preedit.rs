//! 【输入法绘制】【文字装饰】按实际字体下划线度量绘制候选，缩放在像素取整之前完成

use cosmic_text::{Color, LayoutRun, UnderlineStyle};

/// 【输入法绘制】【下划线区域】将 run 中实际装饰按 scale 和 default_color 交给 draw；保留双向文字几何，无返回值
pub(crate) fn draw(
    run: &LayoutRun<'_>,
    scale: f32,
    default_color: Color,
    draw: &mut impl FnMut(i32, i32, u32, u32, Color),
) {
    draw_at(run, scale, (0.0, 0.0), default_color, draw);
}

/// 【输入法绘制】【锚点装饰】把 run 的装饰按 scale、origin 一并转换到设备像素，default_color 为默认前景；draw 接收整数区域，无返回值
pub(crate) fn draw_at(
    run: &LayoutRun<'_>,
    scale: f32,
    origin: (f32, f32),
    default_color: Color,
    draw: &mut impl FnMut(i32, i32, u32, u32, Color),
) {
    for span in run.decorations {
        if span.data.text_decoration.underline == UnderlineStyle::None {
            continue;
        }
        let Some(glyphs) = run.glyphs.get(span.glyph_range.clone()) else {
            continue;
        };
        let left = glyphs
            .iter()
            .map(|glyph| glyph.x)
            .fold(f32::INFINITY, f32::min);
        let right = glyphs
            .iter()
            .map(|glyph| glyph.x + glyph.w)
            .fold(f32::NEG_INFINITY, f32::max);
        if !left.is_finite() || right <= left {
            continue;
        }
        let color = span
            .data
            .text_decoration
            .underline_color_opt
            .or(span.color_opt)
            .unwrap_or(default_color);
        let thickness = (span.data.underline_metrics.thickness * span.font_size * scale)
            .max(1.0)
            .ceil() as u32;
        let top = ((origin.1 + run.line_y - span.data.underline_metrics.offset * span.font_size)
            * scale)
            .floor() as i32;
        let x = ((origin.0 + left) * scale).floor() as i32;
        let width = (((origin.0 + right) * scale).ceil() as i32 - x).max(0) as u32;
        draw(x, top, width, thickness, color);
        if span.data.text_decoration.underline == UnderlineStyle::Double {
            draw(x, top + (thickness * 2) as i32, width, thickness, color);
        }
    }
}
