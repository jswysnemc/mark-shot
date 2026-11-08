// Cropping frames for scrolling capture from screen buffers into stitcher images.

use crate::types::MonitorFrame;
use shot_image::Image;

/// Crops `rect` (x, y, w, h) from `frame` into a new `Image`.
pub fn crop(frame: &MonitorFrame, rect: (u32, u32, u32, u32)) -> Result<Image, String> {
    validate(frame, rect)?;
    let (x, y, w, h) = rect;

    let stride = frame.pw_stride as usize;
    let row_bytes = w as usize * 4;
    let mut pixels = Vec::with_capacity(w as usize * h as usize);
    for row in 0..h as usize {
        let start = (y as usize + row) * stride + x as usize * 4;
        let slice = &frame.pixels[start..start + row_bytes];
        pixels.extend(slice.as_chunks::<4>().0.iter().map(|pixel| {
            (pixel[3] as u32) << 24
                | (pixel[0] as u32) << 16
                | (pixel[1] as u32) << 8
                | pixel[2] as u32
        }));
    }
    Image::from_pixels(w as i32, h as i32, pixels).map_err(|error| error.to_string())
}

/// 【滚动截图】【帧边界】按frame验证rect的尺寸、行距和最后有效字节，返回校验结果，不读取或复制像素
fn validate(frame: &MonitorFrame, rect: (u32, u32, u32, u32)) -> Result<(), String> {
    let (x, y, width, height) = rect;
    if width == 0 || height == 0 {
        return Err("the scroll region has no pixels".into());
    }
    let right = x.checked_add(width).ok_or("scroll region overflows")?;
    let bottom = y.checked_add(height).ok_or("scroll region overflows")?;
    if right > frame.pw_width
        || bottom > frame.pw_height
        || width > i32::MAX as u32
        || height > i32::MAX as u32
    {
        return Err("scroll region is outside the captured frame".into());
    }
    let row_end = u64::from(right) * 4;
    let end = u64::from(bottom - 1) * u64::from(frame.pw_stride) + row_end;
    if row_end > u64::from(frame.pw_stride) || end > frame.pixels.len() as u64 {
        return Err("the capture buffer is smaller than its reported stride".into());
    }
    Ok(())
}

/// 【滚动截图】【原帧签名】直接从frame的rect按cols和rows采样，返回与裁切后完全一致的亮度签名，不分配裁切图像
pub(crate) fn capture_signature(
    frame: &MonitorFrame,
    rect: (u32, u32, u32, u32),
    cols: usize,
    rows: usize,
) -> Result<Vec<u8>, String> {
    validate(frame, rect)?;
    let (left, top, width, height) = rect;
    let mut signature = Vec::with_capacity(cols * rows);
    for row in 0..rows {
        let vertical = top as usize + row * height as usize / rows;
        for column in 0..cols {
            let horizontal = left as usize + column * width as usize / cols;
            let offset = vertical * frame.pw_stride as usize + horizontal * 4;
            let pixel = &frame.pixels[offset..offset + 4];
            signature.push(
                (0.299 * pixel[0] as f32 + 0.587 * pixel[1] as f32 + 0.114 * pixel[2] as f32) as u8,
            );
        }
    }
    Ok(signature)
}

/// Downsamples image luminance across a cols x rows grid to detect duplicate frames.
pub fn signature(image: &Image, cols: usize, rows: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(cols * rows);
    let (w, h) = (image.width(), image.height());
    if w <= 0 || h <= 0 || cols == 0 || rows == 0 {
        return out;
    }
    for row in 0..rows {
        let y = (row as i32 * h / rows as i32).clamp(0, h - 1);
        for col in 0..cols {
            let x = (col as i32 * w / cols as i32).clamp(0, w - 1);
            let pixel = image.row(y)[x as usize];
            // ITU-R BT.601 coefficients
            let r = ((pixel >> 16) & 255) as f32;
            let g = ((pixel >> 8) & 255) as f32;
            let b = (pixel & 255) as f32;
            out.push((0.299 * r + 0.587 * g + 0.114 * b) as u8);
        }
    }
    out
}

/// Checks whether two frame signatures are practically identical.
pub fn is_duplicate(previous: &[u8], current: &[u8]) -> bool {
    if previous.is_empty() || previous.len() != current.len() {
        return false;
    }
    let mut sum = 0u32;
    let mut max = 0u32;
    for (&a, &b) in previous.iter().zip(current) {
        let diff = a.abs_diff(b) as u32;
        sum += diff;
        max = max.max(diff);
    }
    let avg = sum as f32 / previous.len() as f32;
    avg <= 1.1 && max <= 4
}
