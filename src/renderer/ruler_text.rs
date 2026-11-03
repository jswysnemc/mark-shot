//! 【尺子】【刻度文字】一次排版带描边数字，并在小型缓冲中完成旋转

use cosmic_text::{Style, Weight};
use tiny_skia::{Color, FilterQuality, Pixmap, PixmapPaint, Rect, Transform};

use super::Painter;
use super::text::shape_single_line;

/// 【尺子】【数字绘制】将 text 按 size、color 绘入 rect；rotation 围绕 center 旋转整段文字，无返回值
pub(crate) fn halo_number(
    painter: &mut Painter<'_>,
    rect: Rect,
    text: &str,
    size: f32,
    color: Color,
    rotation: f32,
    center: (f32, f32),
) {
    let Some(fonts) = painter.fonts.as_mut() else {
        return;
    };
    let (buffer, width, height) =
        shape_single_line(fonts.system, text, size, Weight::SEMIBOLD, Style::Normal);
    let position = (
        rect.left() + (rect.width() - width) / 2.0,
        rect.top() + (rect.height() - height) / 2.0,
    );
    let padding = (size / 13.0).max(0.8);
    let scale = painter.scale();
    let left = rect.left() - padding;
    let top = rect.top() - padding;
    let Some(mut pixels) = Pixmap::new(
        ((rect.width() + padding * 2.0) * scale).ceil().max(1.0) as u32,
        ((rect.height() + padding * 2.0) * scale).ceil().max(1.0) as u32,
    ) else {
        return;
    };
    let luminance = 0.299 * color.red() + 0.587 * color.green() + 0.114 * color.blue();
    let mut halo = if luminance > 0.55 {
        Color::from_rgba8(8, 13, 19, 200)
    } else {
        Color::from_rgba8(255, 255, 255, 220)
    };
    halo.apply_opacity(color.alpha());
    // 1. 【尺子】【数字绘制】复用同一排版结果绘制八方向描边与正文，缓冲仅覆盖数字范围
    {
        let mut text_painter = painter.sub(&mut pixels).with_origin((left, top));
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
            text_painter.text_buffer(
                &buffer,
                (position.0 + x * padding, position.1 + y * padding),
                halo,
            );
        }
        text_painter.text_buffer(&buffer, position, color);
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
