// Downsampled thumbnails for the scrolling capture preview panel.

use shot_stitch::StitchedView;
use tiny_skia::{Pixmap, PremultipliedColorU8};

/// 【滚动截图】【缩略采样】按 image 与 target_w 返回等比缩略像素，尺寸无效或分配失败时返回 None
pub fn thumbnail(image: StitchedView<'_>, target_w: u32) -> Option<Pixmap> {
    let (w, h) = (image.width(), image.height());
    if w <= 0 || h <= 0 || target_w == 0 {
        return None;
    }
    let scale = target_w as f32 / w as f32;
    let target_h = ((h as f32 * scale).round() as u32).max(1);
    let mut pixmap = Pixmap::new(target_w, target_h)?;
    let dst = pixmap.pixels_mut();

    for y in 0..target_h {
        let sy = ((y as f32 / scale) as i32).clamp(0, h - 1);
        for x in 0..target_w {
            let sx = ((x as f32 / scale) as i32).clamp(0, w - 1);
            let pixel = image.pixel(sx, sy);
            let r = ((pixel >> 16) & 255) as u8;
            let g = ((pixel >> 8) & 255) as u8;
            let b = (pixel & 255) as u8;
            dst[(y * target_w + x) as usize] = PremultipliedColorU8::from_rgba(r, g, b, 255)
                .unwrap_or(PremultipliedColorU8::TRANSPARENT);
        }
    }

    Some(pixmap)
}
