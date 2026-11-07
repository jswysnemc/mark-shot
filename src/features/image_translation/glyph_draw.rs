//! 【图片翻译】【字形栅格】排版与绘制采用同一定点字宽，仅为可见字形分配小缓冲

use crate::theme::Rgba;
use cosmic_text::{FontSystem, LayoutGlyph, SwashCache, SwashContent};
use tiny_skia::{Pixmap, PixmapPaint, PremultipliedColorU8, Transform};

/// 【图片翻译】【字形颜色】按 image 和 color 返回有色字形缓冲，遮罩使用前景色，彩色字形保留自身颜色
fn pixels(image: &cosmic_text::SwashImage, color: Rgba) -> Option<Pixmap> {
    let mut pixmap = Pixmap::new(image.placement.width, image.placement.height)?;
    for (index, pixel) in pixmap.pixels_mut().iter_mut().enumerate() {
        let (rgb, alpha) = match image.content {
            SwashContent::Mask => ([color.0, color.1, color.2], image.data[index]),
            SwashContent::Color => (
                [
                    image.data[index * 4],
                    image.data[index * 4 + 1],
                    image.data[index * 4 + 2],
                ],
                image.data[index * 4 + 3],
            ),
            SwashContent::SubpixelMask => (
                [color.0, color.1, color.2],
                image.data[index * 4..index * 4 + 3]
                    .iter()
                    .copied()
                    .max()
                    .unwrap_or(0),
            ),
        };
        let alpha = (u16::from(alpha) * u16::from(color.3) + 127) / 255;
        let channels = rgb.map(|channel| ((u16::from(channel) * alpha + 127) / 255) as u8);
        *pixel =
            PremultipliedColorU8::from_rgba(channels[0], channels[1], channels[2], alpha as u8)
                .unwrap_or(PremultipliedColorU8::TRANSPARENT);
    }
    Some(pixmap)
}

/// 【图片翻译】【基线绘制】按 glyphs、baseline、offset 和 scale 在 canvas 绘制同一字宽布局，fonts、cache 负责栅格化，屏幕外字形不分配绘制缓冲
pub(super) fn draw(
    canvas: &mut Pixmap,
    glyphs: &[LayoutGlyph],
    baseline: (f32, f32),
    offset: (f32, f32),
    scale: (f32, f32),
    color: Rgba,
    fonts: &mut FontSystem,
    cache: &mut SwashCache,
) {
    let ratio = scale.1 / scale.0;
    for glyph in glyphs {
        let physical = glyph.physical(
            (
                baseline.0 * scale.0 + offset.0,
                (baseline.1 * scale.1 + offset.1) / ratio,
            ),
            scale.0,
        );
        let Some(image) = cache.get_image(fonts, physical.cache_key) else {
            continue;
        };
        let x = physical.x + image.placement.left;
        let y = (physical.y - image.placement.top) as f32 * ratio;
        if x as f32 >= canvas.width() as f32
            || y >= canvas.height() as f32
            || x + image.placement.width as i32 <= 0
            || y + image.placement.height as f32 * ratio <= 0.0
        {
            continue;
        }
        if let Some(pixmap) = pixels(image, color) {
            canvas.draw_pixmap(
                0,
                0,
                pixmap.as_ref(),
                &PixmapPaint::default(),
                Transform::from_scale(1.0, ratio).post_translate(x as f32, y),
                None,
            );
        }
    }
}
