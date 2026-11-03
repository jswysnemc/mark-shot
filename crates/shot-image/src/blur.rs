//! 【图像核心】【高斯模糊】三次滑动盒式模糊，在预乘空间处理全部颜色和透明度

use crate::buffer::{allocate, pixel_count};
use crate::{Image, ImageError, ImageView};

/// 【图像核心】【盒式半径】由 sigma 计算三次盒式半径；返回长度为三的数组
pub fn radii(sigma: f64) -> [i32; 3] {
    let sigma = if sigma.is_finite() {
        sigma.clamp(1.0, 2048.0)
    } else {
        1.0
    };
    let mut lower = (4.0 * sigma * sigma + 1.0).sqrt().floor() as i32;
    if lower % 2 == 0 {
        lower -= 1;
    }
    lower = lower.max(1);
    let count =
        ((12.0 * sigma * sigma - 3.0 * f64::from(lower).powi(2) - 12.0 * f64::from(lower) - 9.0)
            / (-4.0 * f64::from(lower) - 4.0))
            .round() as i32;
    std::array::from_fn(|index| {
        ((if (index as i32) < count {
            lower
        } else {
            lower + 2
        }) - 1)
            / 2
    })
}

/// 【图像核心】【滑动盒式模糊】把 source 写入 target；size 为宽高，radius 为半径，horizontal 为方向，无返回值
fn box_pass(source: &[u32], target: &mut [u32], size: (i32, i32), radius: i32, horizontal: bool) {
    let (width, height) = size;
    let (lines, length) = if horizontal {
        (height, width)
    } else {
        (width, height)
    };
    let window = i64::from(radius * 2 + 1);
    let index = |line: i32, position: i32| {
        if horizontal {
            line as usize * width as usize + position as usize
        } else {
            position as usize * width as usize + line as usize
        }
    };
    for line in 0..lines {
        let mut sum = [0_i64; 4];
        // 1. 【图像核心】【滑动盒式模糊】原图边缘按最近像素扩展
        for offset in -radius..=radius {
            let pixel = source[index(line, offset.clamp(0, length - 1))];
            for (channel, total) in sum.iter_mut().enumerate() {
                *total += i64::from((pixel >> (channel * 8)) & 255);
            }
        }
        // 2. 【图像核心】【滑动盒式模糊】每步仅移入、移出一个像素，复杂度不随半径增加
        for position in 0..length {
            target[index(line, position)] =
                sum.iter().enumerate().fold(0, |pixel, (channel, total)| {
                    pixel | ((*total / window) as u32) << (channel * 8)
                });
            let incoming = source[index(line, (position + radius + 1).min(length - 1))];
            let outgoing = source[index(line, (position - radius).max(0))];
            for (channel, total) in sum.iter_mut().enumerate() {
                *total += i64::from((incoming >> (channel * 8)) & 255)
                    - i64::from((outgoing >> (channel * 8)) & 255);
            }
        }
    }
}

/// 【图像核心】【所有权模糊】消费 width、height 对应的预乘 pixels，以 radius 模糊；返回结果或参数、分配错误
pub fn gaussian_owned(
    mut pixels: Vec<u32>,
    width: i32,
    height: i32,
    radius: i32,
) -> Result<Image, ImageError> {
    if pixel_count(width, height)? != pixels.len() || radius > 4096 {
        return Err(ImageError::Parameter);
    }
    if radius < 1 || width < 2 || height < 2 {
        return Image::from_pixels(width, height, pixels);
    }
    let mut scratch = allocate(pixels.len())?;
    for radius in radii((f64::from(radius) / 2.0).max(1.0)) {
        if radius > 0 {
            box_pass(&pixels, &mut scratch, (width, height), radius, true);
            box_pass(&scratch, &mut pixels, (width, height), radius, false);
        }
    }
    Image::from_pixels(width, height, pixels)
}

/// 【图像核心】【借用模糊】按 radius 模糊预乘 ARGB source；返回结果，零半径或单行、单列原样保留
pub fn gaussian(source: ImageView<'_>, radius: i32) -> Result<Image, ImageError> {
    if radius > 4096 {
        return Err(ImageError::Parameter);
    }
    let mut pixels = allocate(pixel_count(source.width(), source.height())?)?;
    for y in 0..source.height() {
        let start = y as usize * source.width() as usize;
        pixels[start..start + source.width() as usize].copy_from_slice(source.row(y));
    }
    gaussian_owned(pixels, source.width(), source.height(), radius)
}
