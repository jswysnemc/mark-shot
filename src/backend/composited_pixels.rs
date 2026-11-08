//! 【原生帧】【直接合成】前景与冻结背景一次写入原生缓冲，避免透明页回收后立即重新分配

use crate::{AppResult, image_source::ImageSource, types::DamageRect};
use tiny_skia::IntRect;

/// 【原生帧】【合成写入】按size和damage将foreground与background合成为BGRA写入native；返回尺寸校验结果，范围外字节不变
pub(crate) fn write(
    foreground: &[u8],
    background: &ImageSource,
    native: &mut [u8],
    size: (u32, u32),
    damage: Option<DamageRect>,
) -> AppResult {
    let bytes = (size.0 as usize)
        .checked_mul(size.1 as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or("native frame is too large")?;
    if foreground.len() != bytes
        || native.len() != bytes
        || (background.width(), background.height()) != size
    {
        return Err("composited pixels do not match the native frame".into());
    }
    let (left, top, width, height) = damage.unwrap_or((0, 0, size.0, size.1));
    let bounds = IntRect::from_xywh(0, 0, size.0, size.1).ok_or("invalid native size")?;
    let Some(region) = IntRect::from_xywh(left as i32, top as i32, width, height)
        .and_then(|region| region.intersect(&bounds))
    else {
        return Ok(());
    };
    let stride = size.0 as usize * 4;
    background.visit_rows_with_budget(region, 2 * 1024 * 1024, |row, background| {
        let first = row as usize * stride + region.left() as usize * 4;
        let last = first + background.len();
        let front_row = &foreground[first..last];
        if front_row.get(3) == Some(&255)
            && front_row
                .as_chunks::<4>()
                .0
                .iter()
                .fold(255, |alpha, pixel| alpha & pixel[3])
                == 255
        {
            crate::utils::copy_swizzled(&mut native[first..last], front_row);
            return;
        }
        for ((output, front), back) in native[first..last]
            .chunks_mut(64)
            .zip(foreground[first..last].chunks(64))
            .zip(background.chunks(64))
        {
            // 1. 【原生帧】【透明块直传】绝大多数前景块完全透明，直接使用已有向量通道转换，跳过逐像素混合
            if front == &([0; 64][..front.len()]) {
                crate::utils::copy_swizzled(output, back);
                continue;
            }
            // 2. 【原生帧】【不透明直传】缩放场景已经包含完整背景，不再逐像素混合一次旧冻结帧
            if front
                .as_chunks::<4>()
                .0
                .iter()
                .fold(255, |alpha, pixel| alpha & pixel[3])
                == 255
            {
                crate::utils::copy_swizzled(output, front);
                continue;
            }
            for ((output, front), back) in output
                .as_chunks_mut::<4>()
                .0
                .iter_mut()
                .zip(front.as_chunks::<4>().0)
                .zip(back.as_chunks::<4>().0)
            {
                let inverse = 255 - u32::from(front[3]);
                for (channel, source) in [2, 1, 0, 3].into_iter().enumerate() {
                    output[channel] = (u32::from(front[source])
                        + (u32::from(back[source]) * inverse + 127) / 255)
                        .min(255) as u8;
                }
            }
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【原生帧测试】【混合等价】无参数，将透明、半透明和不透明前景的完整及局部提交与原有两步合成逐字节比较，无返回值
    #[test]
    fn direct_composite_matches_swizzle_then_background() {
        let mut foreground = tiny_skia::Pixmap::new(513, 257).unwrap();
        let mut background = foreground.clone();
        for (index, (front, back)) in foreground
            .pixels_mut()
            .iter_mut()
            .zip(background.pixels_mut())
            .enumerate()
        {
            let alpha = match (index / 16) % 3 {
                0 => 0,
                1 => 255,
                _ => index as u8,
            };
            *front = tiny_skia::ColorU8::from_rgba(index as u8, 73, 141, alpha).premultiply();
            *back = tiny_skia::ColorU8::from_rgba(21, index as u8, 97, 255).premultiply();
        }
        let background = ImageSource::memory(background);
        for damage in [None, Some((13, 7, 381, 231)), Some((500, 250, 40, 20))] {
            let mut expected = vec![93; foreground.data().len()];
            crate::backend::wayland::overlay::frame_pixels::FramePixels::Borrowed(
                foreground.data(),
            )
            .write_to(&mut expected, (513, 257), damage)
            .unwrap();
            crate::backend::native_backdrop::compose_bgra(
                &background,
                &mut expected,
                (513, 257),
                damage,
            )
            .unwrap();
            let mut actual = vec![93; expected.len()];
            write(
                foreground.data(),
                &background,
                &mut actual,
                (513, 257),
                damage,
            )
            .unwrap();
            assert_eq!(actual, expected);
        }
    }
}
