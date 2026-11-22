//! 【界面字体】【字形位图】将 FreeType 位图转换为独立且紧凑的遮罩，保存通道顺序供安全合成

use freetype::{Bitmap, bitmap::PixelMode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Content {
    Mask,
    Subpixel,
    Color,
}

pub(crate) struct Glyph {
    pub width: u32,
    pub height: u32,
    pub left: i32,
    pub top: i32,
    pub content: Content,
    pub data: Vec<u8>,
}

/// 【界面字体】【位图转换】按 bitmap、left、top 与 bgr 返回紧凑位图，处理行填充及通道方向，不支持的格式返回空值
pub(super) fn convert(bitmap: &Bitmap, left: i32, top: i32, bgr: bool) -> Option<Glyph> {
    let mode = bitmap.pixel_mode().ok()?;
    let raw_width = bitmap.width().max(0) as usize;
    let raw_height = bitmap.rows().max(0) as usize;
    let (width, height, channels, content) = match mode {
        PixelMode::Mono | PixelMode::Gray => (raw_width, raw_height, 1, Content::Mask),
        PixelMode::Lcd => (raw_width / 3, raw_height, 3, Content::Subpixel),
        PixelMode::LcdV => (raw_width, raw_height / 3, 3, Content::Subpixel),
        PixelMode::Bgra => (raw_width, raw_height, 4, Content::Color),
        _ => return None,
    };
    let mut data = vec![0; width * height * channels];
    let pitch = bitmap.pitch().unsigned_abs() as usize;
    let row = |y: usize| {
        let index = if bitmap.pitch() < 0 {
            raw_height - 1 - y
        } else {
            y
        };
        &bitmap.buffer()[index * pitch..][..pitch]
    };
    // 1. 【界面字体】【格式归一】单色位图展开为二值遮罩，LCD 遮罩保存 RGB 覆盖率，彩色字形保持预乘颜色
    for y in 0..height {
        for x in 0..width {
            let out = &mut data[(y * width + x) * channels..][..channels];
            match mode {
                PixelMode::Mono => {
                    out[0] = if row(y)[x / 8] & (128 >> (x % 8)) != 0 {
                        255
                    } else {
                        0
                    }
                }
                PixelMode::Gray => out[0] = row(y)[x],
                PixelMode::Lcd => out.copy_from_slice(&row(y)[x * 3..][..3]),
                PixelMode::LcdV => {
                    for (channel, value) in out.iter_mut().enumerate() {
                        *value = row(y * 3 + channel)[x];
                    }
                }
                PixelMode::Bgra => {
                    let pixel = &row(y)[x * 4..][..4];
                    out.copy_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
                }
                _ => unreachable!(),
            }
            if bgr && content == Content::Subpixel {
                out.swap(0, 2);
            }
        }
    }
    Some(Glyph {
        width: width as u32,
        height: height as u32,
        left,
        top,
        content,
        data,
    })
}
