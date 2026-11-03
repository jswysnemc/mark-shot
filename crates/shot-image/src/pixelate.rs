//! 【图像核心】【区域像素化】从区域左上角平均预乘通道，不重复乘透明度

use crate::buffer::{allocate, pixel_count};
use crate::{Image, ImageError, ImageView};

/// 【图像核心】【像素块平均】按 block_size 对 source 分块；返回预乘 ARGB 图像，残余块只计算有效像素
pub fn apply(source: ImageView<'_>, block_size: i32) -> Result<Image, ImageError> {
    let block_size = block_size.clamp(2, 96);
    let width = source.width();
    let height = source.height();
    let mut output = allocate(pixel_count(width, height)?)?;
    for top in (0..height).step_by(block_size as usize) {
        let bottom = (top + block_size).min(height);
        for left in (0..width).step_by(block_size as usize) {
            let right = (left + block_size).min(width);
            let mut sums = [0_u64; 4];
            // 1. 【图像核心】【像素块平均】在预乘空间累加，透明像素不会引入隐藏颜色
            for y in top..bottom {
                for &pixel in &source.row(y)[left as usize..right as usize] {
                    for (channel, sum) in sums.iter_mut().enumerate() {
                        *sum += u64::from((pixel >> (channel * 8)) & 255);
                    }
                }
            }
            let count = (right - left) as u64 * (bottom - top) as u64;
            let average = sums.iter().enumerate().fold(0, |pixel, (channel, sum)| {
                pixel | (((sum + count / 2) / count) as u32) << (channel * 8)
            });
            // 2. 【图像核心】【像素块平均】写入已经预乘的平均值，不通过直通颜色再次转换
            for y in top..bottom {
                output[y as usize * width as usize + left as usize
                    ..y as usize * width as usize + right as usize]
                    .fill(average);
            }
        }
    }
    Image::from_pixels(width, height, output)
}
