// Cropping frames for scrolling capture from screen buffers into stitcher images.

use crate::types::MonitorFrame;
use shot_image::Image;

/// Crops `rect` (x, y, w, h) from `frame` into a new `Image`.
pub fn crop(frame: &MonitorFrame, rect: (u32, u32, u32, u32)) -> Result<Image, String> {
    let (x, y, w, h) = rect;
    if w == 0 || h == 0 {
        return Err("the scroll region has no pixels".into());
    }
    if x + w > frame.pw_width || y + h > frame.pw_height {
        return Err(format!(
            "scroll region {w}x{h}+{x}+{y} is outside the captured {}x{}",
            frame.pw_width, frame.pw_height
        ));
    }

    let stride = frame.pw_stride as usize;
    let row_bytes = w as usize * 4;
    let mut pixels = Vec::with_capacity(w as usize * h as usize);
    for row in 0..h as usize {
        let start = (y as usize + row) * stride + x as usize * 4;
        let end = start + row_bytes;
        let slice = frame
            .pixels
            .get(start..end)
            .ok_or("the capture buffer is smaller than its reported stride")?;
        // RGBA byte stream converted to 0xAARRGGBB expected by stitcher
        pixels.extend(
            slice.as_chunks::<4>().0.iter().map(|c| {
                (c[3] as u32) << 24 | (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32
            }),
        );
    }

    Image::from_pixels(w as i32, h as i32, pixels).map_err(|e| e.to_string())
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
