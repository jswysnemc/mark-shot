//! 【图片翻译】【原图取色】有限边缘采样寻找底板色，正文取高对比像素，不继承界面主题

use super::BoxRect;
use crate::theme::Rgba;
use tiny_skia::Pixmap;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Colors {
    pub background: Rgba,
    pub foreground: Rgba,
}

#[derive(Clone, Copy, Default)]
struct Bin {
    total: [u64; 3],
    count: u64,
}

/// 【图片翻译】【像素读取】按 image、x、y 返回存储 RGB，与原版预乘 QImage::pixel 一致，坐标由调用方限制
fn rgb(image: &Pixmap, x: u32, y: u32) -> [u8; 3] {
    let pixel = image.pixels()[(y * image.width() + x) as usize];
    [pixel.red(), pixel.green(), pixel.blue()]
}

/// 【图片翻译】【量化累计】将 color 累计进 bins 的每通道八档颜色格，不保存全部边缘像素
fn add(bins: &mut [Bin; 512], color: [u8; 3]) {
    let key = ((usize::from(color[0]) >> 5) << 6)
        | ((usize::from(color[1]) >> 5) << 3)
        | (usize::from(color[2]) >> 5);
    for (total, channel) in bins[key].total.iter_mut().zip(color) {
        *total += u64::from(channel);
    }
    bins[key].count += 1;
}

/// 【图片翻译】【有限矩形】按 image 和 half-open rect 以原版最大边采样步长累计 bins，无效区域不取样
fn sample_rect(image: &Pixmap, rect: [i32; 4], bins: &mut [Bin; 512]) {
    let [x, y, w, h] = rect;
    if w <= 0 || h <= 0 {
        return;
    }
    for py in (y..y + h).step_by((h / 160).max(1) as usize) {
        for px in (x..x + w).step_by((w / 160).max(1) as usize) {
            add(bins, rgb(image, px as u32, py as u32));
        }
    }
}

/// 【图片翻译】【亮度计算】按 color 返回原版 RGB 加权亮度，不使用界面明暗模式
fn luma(color: [u8; 3]) -> f64 {
    f64::from(color[0]) * 0.299 + f64::from(color[1]) * 0.587 + f64::from(color[2]) * 0.114
}

/// 【图片翻译】【颜色距离】按 left、right 返回 RGB 欧氏距离
fn distance(left: [u8; 3], right: [u8; 3]) -> f64 {
    left.into_iter()
        .zip(right)
        .map(|(a, b)| (f64::from(a) - f64::from(b)).powi(2))
        .sum::<f64>()
        .sqrt()
}

/// 【图片翻译】【原版配色】按 image、rect 和 line_height 返回文字与底板颜色，取样数受步长限制，边缘无外圈时回退内圈
pub fn sample(image: &Pixmap, rect: BoxRect, line_height: f64) -> Colors {
    let mut colors = Colors {
        background: Rgba(255, 255, 255, 255),
        foreground: Rgba(17, 24, 39, 255),
    };
    if !rect.valid() || !line_height.is_finite() {
        return colors;
    }
    let x = rect.left().floor().max(0.0).min(f64::from(image.width())) as i32;
    let y = rect.top().floor().max(0.0).min(f64::from(image.height())) as i32;
    let right = rect.right().ceil().max(0.0).min(f64::from(image.width())) as i32;
    let bottom = rect.bottom().ceil().max(0.0).min(f64::from(image.height())) as i32;
    if right <= x || bottom <= y {
        return colors;
    }
    let margin = ((line_height * 0.15).round() as i32).clamp(2, 6);
    let ox = (x - margin).max(0);
    let oy = (y - margin).max(0);
    let or = (right + margin).min(image.width() as i32);
    let ob = (bottom + margin).min(image.height() as i32);
    let mut bins = [Bin::default(); 512];
    // 1. 【图片翻译】【背景外圈】四条外圈与原版包含端点整数矩形对应，避免重复包含文字内部
    for bounds in [
        [ox, oy, or - ox, y - oy],
        [ox, bottom, or - ox, ob - bottom],
        [ox, y, x - ox, bottom - y],
        [right, y, or - right, bottom - y],
    ] {
        sample_rect(image, bounds, &mut bins);
    }
    if bins.iter().all(|bin| bin.count == 0) {
        sample_rect(image, [x, y, right - x, 1], &mut bins);
        sample_rect(image, [x, bottom - 1, right - x, 1], &mut bins);
    }
    let Some(best) = bins
        .iter()
        .filter(|bin| bin.count > 0)
        .max_by_key(|bin| bin.count)
    else {
        return colors;
    };
    let background = best.total.map(|value| (value / best.count) as u8);
    colors.background = Rgba(background[0], background[1], background[2], 255);
    let (step_x, step_y) = (
        ((right - x) / 64).max(1) as usize,
        ((bottom - y) / 64).max(1) as usize,
    );
    let mut samples = Vec::new();
    let mut maximum = 0.0_f64;
    for py in (y..bottom).step_by(step_y) {
        for px in (x..right).step_by(step_x) {
            let pixel = rgb(image, px as u32, py as u32);
            let dist = distance(pixel, background);
            maximum = maximum.max(dist);
            samples.push((dist, pixel));
        }
    }
    // 2. 【图片翻译】【正文颜色】只累计距离达到最大差别七成的样本，舍弃字形抗锯齿过渡像素
    let mut count = 0_u64;
    let mut total = [0_u64; 3];
    for (dist, pixel) in samples {
        if maximum > 0.0 && dist >= maximum * 0.7 {
            count += 1;
            for (value, channel) in total.iter_mut().zip(pixel) {
                *value += u64::from(channel);
            }
        }
    }
    let mut foreground = if count > 0 {
        total.map(|value| (value / count) as u8)
    } else {
        [17, 24, 39]
    };
    if (luma(foreground) - luma(background)).abs() < 90.0 {
        foreground = if luma(background) > 140.0 {
            [17, 24, 39]
        } else {
            [248, 250, 252]
        };
    }
    colors.foreground = Rgba(foreground[0], foreground[1], foreground[2], 255);
    colors
}
