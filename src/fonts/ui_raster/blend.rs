//! 【界面字体】【遮罩合成】不透明背景按 LCD 通道合成，透明背景采用灰度覆盖率，保持合法预乘颜色

use super::{Content, Glyph};
use tiny_skia::{Color, Pixmap, PremultipliedColorU8};

/// 【界面字体】【像素合成】按 destination、mask 与 color 返回预乘像素，LCD 只在不透明背景分通道计算
fn pixel(destination: PremultipliedColorU8, mask: &[u8], color: Color) -> PremultipliedColorU8 {
    let opacity = color.alpha();
    let gray = mask.iter().map(|value| u32::from(*value)).sum::<u32>() as f32 / mask.len() as f32;
    let channels = if mask.len() == 3 && destination.alpha() == 255 {
        [f32::from(mask[0]), f32::from(mask[1]), f32::from(mask[2])]
    } else {
        [gray; 3]
    };
    let previous = [destination.red(), destination.green(), destination.blue()];
    let foreground = [color.red(), color.green(), color.blue()];
    let mut result = [0; 3];
    for index in 0..3 {
        let alpha = channels[index] / 255.0 * opacity;
        result[index] = (foreground[index] * alpha * 255.0
            + f32::from(previous[index]) * (1.0 - alpha))
            .round() as u8;
    }
    let alpha = if destination.alpha() == 255 {
        255
    } else {
        let alpha = gray / 255.0 * opacity;
        (alpha * 255.0 + f32::from(destination.alpha()) * (1.0 - alpha)).round() as u8
    };
    PremultipliedColorU8::from_rgba(result[0], result[1], result[2], alpha).unwrap_or(destination)
}

/// 【界面字体】【位图绘制】将 glyph 按 position 和 color 合成到 canvas，裁切到实际可见范围，无返回值
pub(crate) fn paint(canvas: &mut Pixmap, glyph: &Glyph, position: (i32, i32), color: Color) {
    let width = canvas.width() as i32;
    let height = canvas.height() as i32;
    let channels = match glyph.content {
        Content::Mask => 1,
        Content::Subpixel => 3,
        Content::Color => 4,
    };
    let left = position.0.max(0);
    let top = position.1.max(0);
    let right = (position.0 + glyph.width as i32).min(width);
    let bottom = (position.1 + glyph.height as i32).min(height);
    for y in top..bottom {
        for x in left..right {
            let source = ((y - position.1) as usize * glyph.width as usize
                + (x - position.0) as usize)
                * channels;
            let mask = &glyph.data[source..][..channels];
            let destination = &mut canvas.pixels_mut()[y as usize * width as usize + x as usize];
            if glyph.content == Content::Color {
                let alpha = f32::from(mask[3]) / 255.0 * color.alpha();
                let inverse = 1.0 - alpha;
                let channel = |value: u8, previous: u8| {
                    (f32::from(value) * color.alpha() + f32::from(previous) * inverse).round() as u8
                };
                *destination = PremultipliedColorU8::from_rgba(
                    channel(mask[0], destination.red()),
                    channel(mask[1], destination.green()),
                    channel(mask[2], destination.blue()),
                    (alpha * 255.0 + f32::from(destination.alpha()) * inverse).round() as u8,
                )
                .unwrap_or(*destination);
            } else {
                *destination = pixel(*destination, mask, color);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// 【界面字体测试】【透明 LCD】无参数，透明背景使用共同覆盖率并保持合法预乘颜色，不透明背景保留 RGB 顺序
    #[test]
    fn transparent_subpixel_masks_remain_valid_and_opaque_masks_keep_channel_order() {
        let white = Color::WHITE;
        let transparent = pixel(PremultipliedColorU8::TRANSPARENT, &[255, 0, 0], white);
        assert_eq!(
            (
                transparent.red(),
                transparent.green(),
                transparent.blue(),
                transparent.alpha()
            ),
            (85, 85, 85, 85)
        );
        let black = PremultipliedColorU8::from_rgba(0, 0, 0, 255).unwrap();
        let opaque = pixel(black, &[255, 0, 0], white);
        assert_eq!(
            (opaque.red(), opaque.green(), opaque.blue(), opaque.alpha()),
            (255, 0, 0, 255)
        );
    }
}
