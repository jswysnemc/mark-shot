//! 【滚动截图】【原图交付】从拼接视图直接转换一次最终像素，不创建完整 ARGB 中间快照

#[cfg(test)]
use shot_stitch::StitchedView;
use tiny_skia::Pixmap;
#[cfg(test)]
use tiny_skia::PremultipliedColorU8;

/// 【滚动截图】【消耗原图】按image创建最终RGBA画布并逐块转换，返回无损原图，不同时保留两份已写入的完整像素
pub(crate) fn consume(image: shot_stitch::StitchedImage) -> Option<Pixmap> {
    let (width, height) = (image.width(), image.height());
    let mut output = Pixmap::new(width.try_into().ok()?, height.try_into().ok()?)?;
    image.write_rgba(output.data_mut()).ok()?;
    Some(output)
}

/// 【滚动截图】【像素转换】按 image 的显示顺序返回无损 RGBA 原图，尺寸无效或分配失败时返回 None
#[cfg(test)]
pub(crate) fn to_pixmap(image: StitchedView<'_>) -> Option<Pixmap> {
    let (width, height) = (image.width(), image.height());
    let mut pixels = Pixmap::new(width.try_into().ok()?, height.try_into().ok()?)?;
    for y in 0..height {
        for x in 0..width {
            let value = image.pixel(x, y);
            pixels.pixels_mut()[(y * width + x) as usize] = PremultipliedColorU8::from_rgba(
                (value >> 16) as u8,
                (value >> 8) as u8,
                value as u8,
                255,
            )?;
        }
    }
    Some(pixels)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【滚动截图测试】【原图转移】无参数，逐块消耗后的双轴像素与独立视图转换完全一致，前插后不能错过首块偏移，无返回值
    #[test]
    fn consuming_rows_matches_borrowed_pixels_in_both_axes() {
        for axis in [shot_stitch::Axis::Vertical, shot_stitch::Axis::Horizontal] {
            let mut stitcher = shot_stitch::Stitcher::new(shot_stitch::Config {
                min_overlap: 32,
                min_append: 10,
                accept_diff: 0.5,
                approx_diff: 0.1,
            })
            .unwrap();
            stitcher.set_axis(axis).unwrap();
            for origin in [100, 145, 190, 235, 280, 325, 100, 60] {
                let data = (origin..origin + 180)
                    .flat_map(|y| {
                        (0..80).map(move |x| {
                            0xff00_0000
                                | (((y as u32).wrapping_mul(2_654_435_761).rotate_left(7)
                                    ^ (x as u32).wrapping_mul(2_246_822_519))
                                    & 0x00ff_ffff)
                        })
                    })
                    .collect();
                let image = shot_image::Image::from_pixels(80, 180, data).unwrap();
                stitcher
                    .push(if axis == shot_stitch::Axis::Horizontal {
                        image.transpose().unwrap()
                    } else {
                        image
                    })
                    .unwrap();
            }
            let expected = to_pixmap(stitcher.image_view().unwrap()).unwrap();
            let image = stitcher.take_image().unwrap();
            assert!(stitcher.image_view().is_none());
            assert_eq!(stitcher.pixel_storage().allocated_bytes, 0);
            let actual = consume(image).unwrap();
            assert_eq!(actual.data(), expected.data(), "axis={axis:?}");
        }
    }
}
