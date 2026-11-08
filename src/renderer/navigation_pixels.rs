//! 【画布导航】【可见像素】轴向变换预计算列坐标，按行分配工作，避免通用路径光栅化

use tiny_skia::{Pixmap, PixmapRef, Transform};

struct Sample {
    first: usize,
    second: usize,
    weight: u32,
    coverage: u32,
}

/// 【画布导航】【通道展开】把 pixel 四个八位通道放入独立十六位槽，返回可同时插值的整数
#[inline]
fn packed(pixel: [u8; 4]) -> u64 {
    let value = u64::from(u32::from_le_bytes(pixel));
    (value & 0x00ff00ff) | ((value & 0xff00ff00) << 24)
}

/// 【画布导航】【四通道插值】按 weight 的二百五十六分之一比例混合 first 与 second，返回四个互不溢出的通道槽
#[inline]
fn interpolate(first: u64, second: u64, weight: u32) -> u64 {
    ((first * u64::from(256 - weight) + second * u64::from(weight) + 0x0080008000800080) >> 8)
        & 0x00ff00ff00ff00ff
}

/// 【画布导航】【通道收回】将 pixel 的独立通道槽恢复为预乘 RGBA，返回四字节颜色
#[inline]
fn unpacked(pixel: u64) -> [u8; 4] {
    (((pixel & 0x00ff00ff) | ((pixel >> 24) & 0xff00ff00)) as u32).to_le_bytes()
}

/// 【画布导航】【采样坐标】将 pixel 按 scale、offset 映射到 length 个原始像素，返回邻居、插值权重和边缘覆盖
fn sample(pixel: usize, scale: f32, offset: f32, length: usize) -> Sample {
    let source = ((pixel as f32 + 0.5 - offset) / scale - 0.5).clamp(0.0, (length - 1) as f32);
    let first = source.floor() as usize;
    let coverage = ((pixel as f32 + 1.0).min(offset + length as f32 * scale)
        - (pixel as f32).max(offset))
    .clamp(0.0, 1.0);
    Sample {
        first,
        second: (first + 1).min(length - 1),
        weight: ((source - first as f32) * 256.0).round() as u32,
        coverage: (coverage * 256.0).round() as u32,
    }
}

/// 【画布导航】【轴向绘制】按 transform 将 source 以双线性插值合成到 canvas；只处理可见矩形，返回是否支持此变换
pub(super) fn draw(canvas: &mut Pixmap, source: PixmapRef<'_>, transform: Transform) -> bool {
    if transform.kx != 0.0
        || transform.ky != 0.0
        || transform.sx <= 0.0
        || transform.sy <= 0.0
        || !transform.is_finite()
    {
        return false;
    }
    let left = transform.tx.floor().clamp(0.0, canvas.width() as f32) as usize;
    let top = transform.ty.floor().clamp(0.0, canvas.height() as f32) as usize;
    let right = (transform.tx + source.width() as f32 * transform.sx)
        .ceil()
        .clamp(0.0, canvas.width() as f32) as usize;
    let bottom = (transform.ty + source.height() as f32 * transform.sy)
        .ceil()
        .clamp(0.0, canvas.height() as f32) as usize;
    if left >= right || top >= bottom {
        return true;
    }
    let columns: Vec<_> = (left..right)
        .map(|column| sample(column, transform.sx, transform.tx, source.width() as usize))
        .collect();
    let target_stride = canvas.width() as usize * 4;
    let source_stride = source.width() as usize * 4;
    let pixels = source.data();
    let render = |(row_index, row): (usize, &mut [u8])| {
        let vertical = sample(
            top + row_index,
            transform.sy,
            transform.ty,
            source.height() as usize,
        );
        let first = pixels[vertical.first * source_stride..][..source_stride]
            .as_chunks::<4>()
            .0;
        let second = pixels[vertical.second * source_stride..][..source_stride]
            .as_chunks::<4>()
            .0;
        for (target, horizontal) in row[left * 4..right * 4]
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(&columns)
        {
            let coverage = horizontal.coverage * vertical.coverage;
            let upper = interpolate(
                packed(first[horizontal.first]),
                packed(first[horizontal.second]),
                horizontal.weight,
            );
            let lower = interpolate(
                packed(second[horizontal.first]),
                packed(second[horizontal.second]),
                horizontal.weight,
            );
            let mut interpolated = unpacked(interpolate(upper, lower, vertical.weight));
            if coverage != 65536 {
                for channel in &mut interpolated {
                    *channel = ((u32::from(*channel) * coverage + 32768) >> 16) as u8;
                }
            }
            if interpolated[3] == 255 {
                *target = interpolated;
            } else {
                let inverse = 255 - u32::from(interpolated[3]);
                for channel in 0..4 {
                    target[channel] = (u32::from(interpolated[channel])
                        + (u32::from(target[channel]) * inverse + 127) / 255)
                        .min(255) as u8;
                }
            }
        }
    };
    let rows = &mut canvas.data_mut()[top * target_stride..bottom * target_stride];
    // 1. 【画布导航】【有界并行】采样与遮罩复用最多四个线程，行带彼此独占，不分配额外图像
    super::parallel_rows::for_each(rows, target_stride, 16, |first_row, band| {
        for (index, row) in band.chunks_mut(target_stride).enumerate() {
            render((first_row + index, row));
        }
    });
    true
}

#[cfg(test)]
#[path = "navigation_pixels_tests.rs"]
mod tests;
