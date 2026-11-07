//! 【尺子】【刻度文字】一次排版带描边数字，并在小型缓冲中完成旋转

use cosmic_text::LayoutRun;
use tiny_skia::{Color, FilterQuality, Pixmap, PixmapPaint, Rect, Transform};

use super::Painter;
use super::text::HAlign;

pub(crate) struct Style {
    pub pixels: f32,
    pub unit: f32,
    pub window: crate::features::annotation_style::window::WindowUnits,
    pub align: HAlign,
}

/// 【尺子】【数字绘制】按style和color将text绘入rect，rotation围绕center旋转完整数字，无返回值
pub(crate) fn halo_number(
    painter: &mut Painter<'_>,
    rect: Rect,
    text: &str,
    color: Color,
    rotation: f32,
    center: (f32, f32),
    style: Style,
) {
    let Some(fonts) = painter.fonts.as_mut() else {
        return;
    };
    let (buffer, mut layout) = crate::fonts::ruler::shape(fonts.system, text, style.pixels);
    let factor = style.window.canvas(1.0);
    let metrics = crate::fonts::ruler::measure(text, style.pixels);
    let width = metrics.width * factor;
    let height = metrics.height * factor;
    for glyph in &mut layout.glyphs {
        glyph.x *= factor;
        glyph.y *= factor;
        glyph.w *= factor;
        glyph.font_size *= factor;
    }
    let position = (
        match style.align {
            HAlign::Left => rect.left(),
            HAlign::Center => rect.left() + (rect.width() - width) / 2.0,
            HAlign::Right => rect.right() - width,
        },
        rect.top() + (rect.height() - height) / 2.0,
    );
    let padding = style.unit.max(style.window.canvas(0.8));
    let scale = painter.scale();
    let left = position.0 - padding;
    let top = position.1 - padding;
    let Some(mut pixels) = Pixmap::new(
        ((width + padding * 2.0) * scale).ceil().max(1.0) as u32,
        ((height + padding * 2.0) * scale).ceil().max(1.0) as u32,
    ) else {
        return;
    };
    let luminance = 0.299 * color.red() + 0.587 * color.green() + 0.114 * color.blue();
    let halo = if luminance > 0.55 {
        Color::from_rgba8(8, 13, 19, 200)
    } else {
        Color::from_rgba8(255, 255, 255, 220)
    };
    // 1. 【尺子】【数字绘制】复用同一排版结果绘制八方向描边与正文，缓冲仅覆盖数字范围
    {
        let mut text_painter = painter.sub(&mut pixels).with_origin((left, top));
        let Some(raw) = buffer.layout_runs().next() else {
            return;
        };
        let run = LayoutRun {
            glyphs: &layout.glyphs,
            decorations: &layout.decorations,
            line_y: metrics.ascent * factor,
            line_w: width,
            line_height: height,
            ..raw
        };
        for (x, y) in [
            (-1.0, -1.0),
            (0.0, -1.0),
            (1.0, -1.0),
            (-1.0, 0.0),
            (1.0, 0.0),
            (-1.0, 1.0),
            (0.0, 1.0),
            (1.0, 1.0),
        ] {
            text_painter.text_layout_run(
                &run,
                (position.0 + x * padding, position.1 + y * padding),
                halo,
            );
        }
        text_painter.text_layout_run(&run, position, color);
    }
    // 2. 【尺子】【数字绘制】在物理像素空间旋转小缓冲，角度与长度尺刻度保持一致
    let (sin, cos) = rotation.to_radians().sin_cos();
    let base = painter.to_px(left, top);
    let center = painter.to_px(center.0, center.1);
    let delta = (base.0 - center.0, base.1 - center.1);
    let transform = Transform::from_row(
        cos,
        sin,
        -sin,
        cos,
        center.0 + cos * delta.0 - sin * delta.1,
        center.1 + sin * delta.0 + cos * delta.1,
    );
    painter.canvas.draw_pixmap(
        0,
        0,
        pixels.as_ref(),
        &PixmapPaint {
            quality: FilterQuality::Bilinear,
            ..PixmapPaint::default()
        },
        transform,
        None,
    );
}
