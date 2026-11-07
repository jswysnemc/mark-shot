//! 【区域识别】【双线性采样】普通 ARGB 小图的十六位预乘绘制与定点坐标

use std::borrow::Cow;

use super::Rgb;

/// 【区域识别】【十六位颜色】按 image、x、y 读取预乘通道，返回 RGBA 十六位值
fn sample(image: &Rgb<'_>, x: usize, y: usize) -> [u32; 4] {
    let pixel = &image.pixels[(y * image.width + x) * 4..][..4];
    let alpha = u32::from(pixel[3]) * 257;
    std::array::from_fn(|channel| {
        if channel == 3 {
            alpha
        } else {
            (u32::from(pixel[channel]) * 257 * alpha + 32767) / 65535
        }
    })
}

/// 【区域识别】【定点混合】按 first、second 与 weight 分别截断两项乘积，返回与绘制器一致的通道
fn blend(first: [u32; 4], second: [u32; 4], weight: u32) -> [u32; 4] {
    if weight == 0 {
        return first;
    }
    std::array::from_fn(|channel| {
        ((first[channel] * (65536 - weight)) >> 16) + ((second[channel] * weight) >> 16)
    })
}

/// 【区域识别】【像素边界】按 position 和 limit 计算相邻像素与小数权重，返回图像内位置
fn point(position: i64, limit: usize) -> (usize, usize, u32) {
    let first = position >> 16;
    (
        first.clamp(0, limit as i64 - 1) as usize,
        (first + 1).clamp(0, limit as i64 - 1) as usize,
        (position & 65535) as u32,
    )
}

/// 【区域识别】【八位转换】按 pixel 解除十六位预乘并转换到八位 RGBA，返回实际绘制颜色
fn normal(pixel: [u32; 4]) -> [u8; 4] {
    let alpha = pixel[3];
    if alpha == 0 {
        return [0; 4];
    }
    let factor = 255.0f32 / alpha as f32;
    std::array::from_fn(|channel| {
        if channel == 3 {
            ((alpha + 128 - ((alpha + 128) >> 8)) >> 8) as u8
        } else {
            (pixel[channel] as f32 * factor)
                .round_ties_even()
                .clamp(0.0, 255.0) as u8
        }
    })
}

/// 【区域识别】【普通图像缩图】按 image、width、height 执行十六位双线性采样，返回正常 RGBA 缓冲
pub(super) fn downscale(image: &Rgb<'_>, width: usize, height: usize) -> Rgb<'static> {
    let inverse_x = image.width as f64 / width as f64;
    let inverse_y = image.height as f64 / height as f64;
    let increment = (inverse_x * 65536.0) as i64;
    let mut pixels = vec![0; width * height * 4];
    // 1. 【区域识别】【坐标舍入】绘制器先平移一个定点单位，再反变换；每个二千零四十八像素片段重新建立起点
    for y in 0..height {
        let position_y = ((y as f64 + 0.5) * inverse_y * 65536.0 - 1.0) as i64 - 32768;
        let (y0, y1, weight_y) = point(position_y, image.height);
        for x in 0..width {
            let start = x / 2048 * 2048;
            let position_x = ((start as f64 + 0.5) * inverse_x * 65536.0 - 1.0) as i64 - 32768
                + (x - start) as i64 * increment;
            let (x0, x1, weight_x) = point(position_x, image.width);
            let first = blend(sample(image, x0, y0), sample(image, x0, y1), weight_y);
            let second = blend(sample(image, x1, y0), sample(image, x1, y1), weight_y);
            let pixel = normal(blend(first, second, weight_x));
            pixels[(y * width + x) * 4..][..4].copy_from_slice(&pixel);
        }
    }
    Rgb {
        width,
        height,
        pixels: Cow::Owned(pixels),
        area_scaling: false,
    }
}
