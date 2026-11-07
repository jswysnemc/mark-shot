//! 【区域识别】【方向边缘】色差梯度、弱直线提升和 Canny 空隙补充

use super::image::{Gray, Rgb, canny, gray::difference, morphology};

pub(super) struct Field {
    pub vertical: Gray,
    pub horizontal: Gray,
    pub combined: Gray,
    pub threshold: u8,
}

/// 【区域识别】【弱直线】按 gradient、other 和 vertical 扫描连续弱方向梯度，返回长度至少二十四像素的掩码
fn weak_lines(gradient: &Gray, other: &Gray, vertical: bool) -> Gray {
    let mut mask = Gray::new(gradient.width, gradient.height);
    let (across, along) = if vertical {
        (gradient.width, gradient.height)
    } else {
        (gradient.height, gradient.width)
    };
    for fixed in 0..across {
        let mut start = None;
        for position in 0..=along {
            let (x, y) = if vertical {
                (fixed, position)
            } else {
                (position, fixed)
            };
            let hit = position < along
                && gradient.at(x, y) >= 5
                && u32::from(gradient.at(x, y)) * 4 >= u32::from(other.at(x, y)) * 3;
            if hit {
                if start.is_none() {
                    start = Some(position);
                }
            } else if let Some(start) = start.take()
                && position - start >= 24
            {
                for value in start..position {
                    let (x, y) = if vertical {
                        (fixed, value)
                    } else {
                        (value, fixed)
                    };
                    mask.pixels[y * mask.width + x] = 255;
                }
            }
        }
    }
    mask
}

/// 【区域识别】【方向场构造】按 image 计算方向和综合边缘，返回固定分析分辨率的边缘场
pub(super) fn build(image: &Rgb<'_>) -> Field {
    let (width, height) = (image.width, image.height);
    let mut gx = Gray::new(width, height);
    let mut gy = Gray::new(width, height);
    let mut histogram = [0u32; 256];
    if width >= 3 && height >= 3 {
        for y in 1..height - 1 {
            for x in 1..width - 1 {
                let center = image.at(x, y);
                let dx = difference(image.at(x - 1, y), center)
                    .max(difference(center, image.at(x + 1, y)));
                let dy = difference(image.at(x, y - 1), center)
                    .max(difference(center, image.at(x, y + 1)));
                gx.pixels[y * width + x] = dx;
                gy.pixels[y * width + x] = dy;
                if dx.max(dy) > 2 {
                    histogram[dx.max(dy) as usize] += 1;
                }
            }
        }
    }
    // 1. 【区域识别】【自适应阈值】取非平坦梯度的百分之三十五分位，弱边框阈值限制在五至十八
    let count: u32 = histogram.iter().sum();
    let mut accumulated = 0;
    let mut percentile = 12;
    for (value, frequency) in histogram.iter().enumerate() {
        accumulated += frequency;
        if accumulated > count * 35 / 100 {
            percentile = value as u8;
            break;
        }
    }
    let threshold = (percentile / 2).clamp(5, 18);
    let weak_vertical = weak_lines(&gx, &gy, true);
    let weak_horizontal = weak_lines(&gy, &gx, false);
    let high = (f32::from(threshold) * 3.0).clamp(15.0, 54.0);
    let canny = canny::detect(&image.grayscale(), high * 0.4, high);
    let mut vertical = Gray::new(width, height);
    let mut horizontal = Gray::new(width, height);
    let mut combined = Gray::new(width, height);
    // 2. 【区域识别】【方向抑制】保留台阶单侧极值，避免两像素梯度扩大控件外接框
    for y in 0..height {
        for x in 0..width {
            let index = y * width + x;
            let (dx, dy) = (gx.pixels[index], gy.pixels[index]);
            let v_peak = dx >= gx.outside_zero(x as i32 - 1, y as i32)
                && dx > gx.outside_zero(x as i32 + 1, y as i32);
            let h_peak = dy >= gy.outside_zero(x as i32, y as i32 - 1)
                && dy > gy.outside_zero(x as i32, y as i32 + 1);
            let v_hit = v_peak
                && (dx >= threshold && u32::from(dx) * 4 >= u32::from(dy) * 3
                    || weak_vertical.pixels[index] != 0);
            let h_hit = h_peak
                && (dy >= threshold && u32::from(dy) * 4 >= u32::from(dx) * 3
                    || weak_horizontal.pixels[index] != 0);
            vertical.pixels[index] = if v_hit { 255 } else { 0 };
            horizontal.pixels[index] = if h_hit { 255 } else { 0 };
            combined.pixels[index] = if v_hit || h_hit { 255 } else { 0 };
        }
    }
    // 3. 【区域识别】【空隙补充】Canny 只补充色差边两像素之外的空隙，不在强边周围生成光晕
    let dilated = morphology::dilate(&morphology::dilate(&combined));
    for index in 0..width * height {
        if canny.pixels[index] != 0
            && dilated.pixels[index] == 0
            && gx.pixels[index].max(gy.pixels[index]) < threshold
        {
            combined.pixels[index] = 255;
        }
    }
    Field {
        vertical,
        horizontal,
        combined,
        threshold,
    }
}
