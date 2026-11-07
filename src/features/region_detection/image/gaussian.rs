//! 【区域识别】【高斯模糊】半径三、sigma 1.4 的可分离卷积

use super::Gray;
use std::sync::OnceLock;

/// 【区域识别】【高斯核】无参数，返回一次初始化的七项归一化卷积核
fn kernel() -> &'static [f32; 7] {
    static KERNEL: OnceLock<[f32; 7]> = OnceLock::new();
    KERNEL.get_or_init(|| {
        let mut values = [0.0; 7];
        let mut sum = 0.0f32;
        for (index, value) in values.iter_mut().enumerate() {
            let distance = index as i32 - 3;
            *value = (-(distance * distance) as f32 / (2.0 * 1.4 * 1.4)).exp();
            sum += *value;
        }
        for value in &mut values {
            *value /= sum;
        }
        values
    })
}

/// 【区域识别】【模糊图像】对 image 先水平再垂直卷积，返回按整数四舍五入量化的灰度图
pub(in crate::features::region_detection) fn blur(image: &Gray) -> Gray {
    let (width, height) = (image.width, image.height);
    let mut output = Gray::new(width, height);
    if width == 0 || height == 0 {
        return output;
    }
    let mut horizontal = vec![0.0f32; width * height];
    let kernel = kernel();
    // 1. 【区域识别】【水平卷积】只在两端钳制位置，按原有卷积顺序累加单精度浮点数
    for y in 0..height {
        for x in 0..width {
            let mut sum = 0.0f32;
            for (index, coefficient) in kernel.iter().enumerate() {
                let sample = (x as i32 + index as i32 - 3).clamp(0, width as i32 - 1) as usize;
                sum += f32::from(image.at(sample, y)) * coefficient;
            }
            horizontal[y * width + x] = sum;
        }
    }
    // 2. 【区域识别】【垂直卷积】保留相同边界处理和八位量化，不改变后续 Canny 阈值
    for y in 0..height {
        for x in 0..width {
            let mut sum = 0.0f32;
            for (index, coefficient) in kernel.iter().enumerate() {
                let sample = (y as i32 + index as i32 - 3).clamp(0, height as i32 - 1) as usize;
                sum += horizontal[sample * width + x] * coefficient;
            }
            output.pixels[y * width + x] = sum.round().clamp(0.0, 255.0) as u8;
        }
    }
    output
}
