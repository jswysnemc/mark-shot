//! 【图像核心】【颜色滤镜】按直通颜色计算灰度、反色和提亮，结果使用预乘 ARGB

use crate::buffer::{allocate, pixel_count};
use crate::{Image, ImageError, ImageView};

#[derive(Clone, Copy, Debug)]
pub enum Filter {
    Grayscale,
    Invert,
    Brighten(f64),
}

/// 【图像核心】【通道预乘】把 value 乘以 alpha；返回与原版八位舍入规则一致的通道
fn premultiply(value: u32, alpha: u32) -> u32 {
    let product = value * alpha + 128;
    (product + (product >> 8)) >> 8
}

/// 【图像核心】【像素变换】对直通 ARGB pixel 应用 filter；返回预乘像素，调用方先验证提亮比例
pub fn transform(pixel: u32, filter: Filter) -> u32 {
    let alpha = pixel >> 24;
    let rgb = [(pixel >> 16) & 255, (pixel >> 8) & 255, pixel & 255];
    let mapped = match filter {
        Filter::Grayscale => {
            let gray = (rgb[0] * 11 + rgb[1] * 16 + rgb[2] * 5) / 32;
            [gray; 3]
        }
        Filter::Invert => rgb.map(|channel| 255 - channel),
        Filter::Brighten(amount) => {
            let amount = if amount.is_finite() {
                amount.clamp(0.0, 1.0)
            } else {
                0.0
            };
            rgb.map(|channel| {
                (f64::from(channel) + f64::from(255 - channel) * amount).round() as u32
            })
        }
    }
    .map(|channel| premultiply(channel, alpha));
    (alpha << 24) | (mapped[0] << 16) | (mapped[1] << 8) | mapped[2]
}

/// 【图像核心】【颜色滤镜】对直通 ARGB source 应用 filter；返回保留透明度的预乘图像或参数错误
pub fn apply(source: ImageView<'_>, filter: Filter) -> Result<Image, ImageError> {
    if matches!(filter, Filter::Brighten(amount) if !amount.is_finite()) {
        return Err(ImageError::Parameter);
    }
    let mut pixels = allocate(pixel_count(source.width(), source.height())?)?;
    for y in 0..source.height() {
        for (x, &pixel) in source.row(y).iter().enumerate() {
            pixels[y as usize * source.width() as usize + x] = transform(pixel, filter);
        }
    }
    Image::from_pixels(source.width(), source.height(), pixels)
}

/// 【图像核心】【模糊强度】把 strength 转为一至四十八像素半径；非有限值返回最小半径
pub fn blur_radius(strength: f64) -> i32 {
    if strength.is_finite() {
        strength.round().clamp(1.0, 48.0) as i32
    } else {
        1
    }
}

/// 【图像核心】【提亮强度】把 strength 转为 0.05 至 0.9 的提亮比例；非有限值返回最小比例
pub fn brightness_amount(strength: f64) -> f64 {
    if strength.is_finite() {
        (strength / 60.0).clamp(0.05, 0.9)
    } else {
        0.05
    }
}
