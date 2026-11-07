//! 【区域识别】【分析缩图】十四位面积权重和横向中间量化，与原版分析采样保持一致

use super::{Rgb, gray::normal_channel};
use std::borrow::Cow;

struct Axis {
    start: usize,
    weights: Vec<u32>,
}

/// 【区域识别】【面积权重】按 source、target 生成单轴采样位置与十四位权重，返回每个目标像素的采样表
fn axis(source: usize, target: usize) -> Vec<Axis> {
    let increment = ((source as u64) << 16) / target as u64;
    let coefficient = ((target as u64 * 16384).div_ceil(source as u64)) as u32;
    (0..target)
        .map(|index| {
            let position = index as u64 * increment;
            let first = (((65536 - (position & 65535)) * u64::from(coefficient)) >> 16) as u32;
            let mut weights = vec![first];
            let mut remaining = 16384 - first;
            while remaining > coefficient {
                weights.push(coefficient);
                remaining -= coefficient;
            }
            weights.push(remaining);
            Axis {
                start: (position >> 16) as usize,
                weights,
            }
        })
        .collect()
}

/// 【区域识别】【预乘取样】按 image、x、y 返回缩图所需预乘 RGBA，完全不透明像素直接读取
fn sample(image: &Rgb<'_>, x: usize, y: usize) -> [u32; 4] {
    let index = (y * image.width + x) * 4;
    let alpha = u32::from(image.pixels[index + 3]);
    let mut color = [0; 4];
    for (channel, output) in color[..3].iter_mut().enumerate() {
        let input = u32::from(image.pixels[index + channel]);
        *output = if alpha == 255 {
            input
        } else {
            (input * alpha + 127) / 255
        };
    }
    color[3] = alpha;
    color
}

/// 【区域识别】【水平面积】按 image、x 和 y 累加一行预乘 RGBA，返回十四位加权通道
fn row(image: &Rgb<'_>, x: &Axis, y: usize) -> [u64; 4] {
    let mut sum = [0; 4];
    for (offset, weight) in x.weights.iter().enumerate() {
        if *weight == 0 {
            continue;
        }
        let pixel = sample(image, x.start + offset, y);
        for channel in 0..4 {
            sum[channel] += u64::from(pixel[channel]) * u64::from(*weight);
        }
    }
    sum
}

/// 【区域识别】【定点缩图】按 image、width、height 执行缩小或恒等采样，返回正常 RGB 视图；同尺寸借用原图
pub(in crate::features::region_detection) fn downscale<'a>(
    image: &'a Rgb<'_>,
    width: usize,
    height: usize,
) -> Rgb<'a> {
    if width == image.width && height == image.height {
        return Rgb {
            width,
            height,
            pixels: Cow::Borrowed(&image.pixels),
            area_scaling: image.area_scaling,
        };
    }
    assert!(width > 0 && height > 0 && width <= image.width && height <= image.height);
    // 1. 【区域识别】【采样路径】普通 ARGB 小图在两倍以内缩小使用绘制器，不透明预乘截图始终使用面积采样
    if !image.area_scaling
        && width * 2 >= image.width
        && height * 2 >= image.height
        && image.width * image.height < 1 << 20
    {
        return super::bilinear::downscale(image, width, height);
    }
    let horizontal = axis(image.width, width);
    let vertical = axis(image.height, height);
    let mut pixels = vec![0; width * height * 4];
    // 1. 【区域识别】【二维面积】水平结果先舍弃四位，再以垂直十四位权重累计，保持原有量化位置
    for (y, vertical) in vertical.iter().enumerate() {
        for (x, horizontal) in horizontal.iter().enumerate() {
            let mut sum = [0u64; 4];
            for (offset, weight) in vertical.weights.iter().enumerate() {
                if *weight == 0 {
                    continue;
                }
                let row = row(image, horizontal, vertical.start + offset);
                for channel in 0..4 {
                    sum[channel] += (row[channel] >> 4) * u64::from(*weight);
                }
            }
            let output = &mut pixels[(y * width + x) * 4..(y * width + x + 1) * 4];
            for channel in 0..4 {
                output[channel] = (sum[channel] >> 24) as u8;
            }
            // 2. 【区域识别】【正常颜色】采样在预乘空间完成，再解除预乘供方向色差与灰度读取
            let alpha = u32::from(output[3]);
            if alpha != 255 {
                for channel in &mut output[..3] {
                    *channel = normal_channel(*channel, alpha);
                }
            }
        }
    }
    Rgb {
        width,
        height,
        pixels: Cow::Owned(pixels),
        area_scaling: false,
    }
}
