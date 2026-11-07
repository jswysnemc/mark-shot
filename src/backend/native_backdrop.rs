//! 【原生帧】【冻结背景】在BGRA原生缓冲中保持原有预乘合成，冻结来源按行访问和回收

use crate::{image_source::ImageSource, types::DamageRect};
use tiny_skia::IntRect;

const READ_BYTES: usize = 2 * 1024 * 1024;

/// 【原生帧】【背景合成】按source、pixels、size和damage把同网格RGBA背景合成到BGRA前景下方，返回校验结果，不修改损坏范围外像素
pub(crate) fn compose_bgra(
    source: &ImageSource,
    pixels: &mut [u8],
    size: (u32, u32),
    damage: Option<DamageRect>,
) -> crate::AppResult {
    let (width, height) = size;
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|count| count.checked_mul(4))
        .ok_or("native frame is too large")?;
    if (source.width(), source.height()) != size || pixels.len() != expected {
        return Err("native backdrop requires the original pixel grid".into());
    }
    let (x, y, w, h) = damage.unwrap_or((0, 0, width, height));
    let bounds = IntRect::from_xywh(0, 0, width, height).ok_or("invalid native frame size")?;
    let Some(region) =
        IntRect::from_xywh(x as i32, y as i32, w, h).and_then(|rect| rect.intersect(&bounds))
    else {
        return Ok(());
    };
    let stride = width as usize * 4;
    source.visit_rows_with_budget(region, READ_BYTES, |row, background| {
        let start = row as usize * stride + region.left() as usize * 4;
        let foreground = &mut pixels[start..start + background.len()];
        // 1. 【原生帧】【预乘顺序】原始前景保持RGBA到BGRA转换结果，仅在前景未完全覆盖时执行DestinationOver
        for (foreground, background) in foreground
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(background.as_chunks::<4>().0)
        {
            let inverse = 255 - u32::from(foreground[3]);
            if inverse == 0 {
                continue;
            }
            if inverse == 255 {
                *foreground = [background[2], background[1], background[0], background[3]];
                continue;
            }
            for (channel, source_channel) in [2, 1, 0, 3].into_iter().enumerate() {
                foreground[channel] = (u32::from(foreground[channel])
                    + (u32::from(background[source_channel]) * inverse + 127) / 255)
                    .min(255) as u8;
            }
        }
    });
    Ok(())
}

#[cfg(test)]
#[path = "native_backdrop_tests.rs"]
mod tests;
