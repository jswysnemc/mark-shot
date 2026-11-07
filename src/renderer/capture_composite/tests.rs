//! 【截图裁切回归】【分数比例】验证原始缓冲尺寸不能按取整逻辑宽高独立重采样

use super::*;
use crate::types::{Capture, Placement};

/// 【截图裁切回归】【原始像素】按 scale 和 logical 返回独立彩色像素截图与对应布局，物理尺寸不要求整除逻辑尺寸
fn desktop(scale: f32, logical: (i32, i32)) -> (Capture, Placement) {
    let mut pixels = Pixmap::new(1280, 800).unwrap();
    for (index, pixel) in pixels.pixels_mut().iter_mut().enumerate() {
        *pixel = tiny_skia::PremultipliedColorU8::from_rgba(
            (index % 251) as u8,
            (index / 1280 % 241) as u8,
            (index % 237) as u8,
            255,
        )
        .unwrap();
    }
    (
        Capture {
            pixmap: pixels.into(),
            scale,
        },
        Placement {
            size: logical,
            position: (0, 0),
            scale: 1.5,
        },
    )
}

/// 【截图裁切回归】【原生比例】非整除输出的候选裁切保留所有冻结像素，包括距右下边界较近的区域，无参数、无返回值
#[test]
fn fractional_native_crops_keep_frozen_pixels() {
    for scale in [1.5, 1280.0 / 853.0] {
        let (capture, placement) = desktop(scale, (853, 533));
        for (x, y, width, height) in [(232, 340, 180, 120), (1010, 650, 180, 120)] {
            let region = Rect::from_xywh(
                x as f32 / scale,
                y as f32 / scale,
                width as f32 / scale,
                height as f32 / scale,
            )
            .unwrap();
            let (actual, _, _) = composite_precise(
                std::slice::from_ref(&capture),
                std::slice::from_ref(&placement),
                region,
                None,
            )
            .unwrap();
            assert_eq!((actual.width(), actual.height()), (width, height));
            for row in 0..height as usize {
                let start = ((y as usize + row) * 1280 + x as usize) * 4;
                assert_eq!(
                    &actual.data()[row * width as usize * 4..(row + 1) * width as usize * 4],
                    &capture.pixmap.data()[start..start + width as usize * 4],
                    "scale={scale}, crop=({x},{y},{width},{height}), row={row}"
                );
            }
        }
    }
}
