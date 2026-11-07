use crate::{ImageError, ImageView, blur, buffer::pixel_count};

use super::{FrameOptions, Layout, mask::RoundedMask, pixels};

/// 为 count 个 16 位透明度值分配零缓冲区，返回缓冲区或分配错误。
fn allocate(count: usize) -> Result<Vec<u16>, ImageError> {
    let mut output = Vec::new();
    output
        .try_reserve_exact(count)
        .map_err(|_| ImageError::Allocation)?;
    output.resize(count, 0);
    Ok(output)
}

/// 对 source 执行单轴盒式模糊到 target，size 为尺寸、radius 为半径，horizontal 指定方向，无返回值。
fn box_pass(source: &[u16], target: &mut [u16], size: (i32, i32), radius: i32, horizontal: bool) {
    let (width, height) = size;
    let (lines, length) = if horizontal {
        (height, width)
    } else {
        (width, height)
    };
    let window = u64::try_from(radius * 2 + 1).expect("validated blur radius");
    let index = |line: i32, position: i32| -> usize {
        if horizontal {
            line as usize * width as usize + position as usize
        } else {
            position as usize * width as usize + line as usize
        }
    };
    for line in 0..lines {
        let mut sum: u64 = (0..=radius.min(length - 1))
            .map(|x| u64::from(source[index(line, x)]))
            .sum();
        for position in 0..length {
            target[index(line, position)] = ((sum + window / 2) / window) as u16;
            let incoming = position + radius + 1;
            let outgoing = position - radius;
            if incoming < length {
                sum += u64::from(source[index(line, incoming)]);
            }
            if outgoing >= 0 {
                sum -= u64::from(source[index(line, outgoing)]);
            }
        }
    }
}

/// 【图像核心】【导出阴影】从 source/mask 生成阴影写入 output，layout/options 指定布局，返回处理结果。
pub(super) fn draw(
    output: &mut [u32],
    source: ImageView<'_>,
    mask: &RoundedMask,
    layout: Layout,
    options: &FrameOptions,
) -> Result<(), ImageError> {
    if options.shadow_radius <= 0 || options.shadow_opacity <= 0.0 {
        return Ok(());
    }
    let radii = blur::radii((f64::from(options.shadow_radius) / 2.0).max(1.0));
    let margin = radii.iter().sum::<i32>() + 1;
    let width = source
        .width()
        .checked_add(margin * 2)
        .ok_or(ImageError::Dimensions)?;
    let height = source
        .height()
        .checked_add(margin * 2)
        .ok_or(ImageError::Dimensions)?;
    let mut alpha = allocate(pixel_count(width, height)?)?;
    // 1. 【图像核心】【导出阴影】在原图周围预留透明边界，避免裁切后模糊导致边缘加深
    for y in 0..source.height() {
        for (x, pixel) in source.row(y).iter().enumerate() {
            alpha[(y + margin) as usize * width as usize + x + margin as usize] =
                ((mask.apply(*pixel, x as i32, y) >> 24) * 257) as u16;
        }
    }
    // 2. 【图像核心】【导出阴影】只模糊透明度，16 位中间值防止多次舍入截断柔和边缘
    let mut scratch = allocate(alpha.len())?;
    for radius in radii {
        if radius > 0 {
            box_pass(&alpha, &mut scratch, (width, height), radius, true);
            box_pass(&scratch, &mut alpha, (width, height), radius, false);
        }
    }
    let opacity = options.shadow_opacity.clamp(0.0, 1.0).sqrt();
    // 3. 【图像核心】【导出阴影】模糊完成后应用偏移与画布裁切，内容在另一阶段只绘制一次
    for y in 0..layout.height {
        let sy = i64::from(y) - i64::from(layout.padding) - i64::from(options.shadow_offset_y)
            + i64::from(margin);
        if !(0..i64::from(height)).contains(&sy) {
            continue;
        }
        for x in 0..layout.width {
            let sx = i64::from(x) - i64::from(layout.padding) + i64::from(margin);
            if (0..i64::from(width)).contains(&sx) {
                let value = alpha[sy as usize * width as usize + sx as usize];
                let value = (f64::from(value) / 257.0 * opacity).round() as u32;
                output[y as usize * layout.width as usize + x as usize] =
                    pixels::shadow_color(options.shadow_color, value);
            }
        }
    }
    Ok(())
}
