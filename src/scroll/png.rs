//! 【滚动截图】【逐行编码】直接读取完整原图视图，编码期间只创建一行RGBA和有界压缩缓冲

use shot_stitch::StitchedView;
use std::io::Write;

const STREAM_BYTES: usize = 64 * 1024;

/// 【滚动截图】【完整PNG】按source逐行编码全部原始像素，返回PNG或编码错误，不创建完整RGBA中间图
pub(crate) fn encode(source: StitchedView<'_>) -> Result<Vec<u8>, String> {
    let width = u32::try_from(source.width()).map_err(|_| "invalid scroll width")?;
    let height = u32::try_from(source.height()).map_err(|_| "invalid scroll height")?;
    encode_rows(width, height, |stream| {
        let mut row = vec![0; width as usize * 4];
        for row_index in 0..source.height() {
            for column in 0..source.width() {
                let value = source.pixel(column, row_index);
                row[column as usize * 4..column as usize * 4 + 4].copy_from_slice(&rgba(value));
            }
            stream.write_all(&row).map_err(|error| error.to_string())?;
        }
        Ok(())
    })
}

/// 【滚动截图】【消费编码】编码 image 并及时释放已输出的纵向像素块，返回无损 PNG；不再需要恢复捕获会话时使用
pub(crate) fn consume(image: shot_stitch::StitchedImage) -> Result<Vec<u8>, String> {
    let width = u32::try_from(image.width()).map_err(|_| "invalid scroll width")?;
    let height = u32::try_from(image.height()).map_err(|_| "invalid scroll height")?;
    encode_rows(width, height, |stream| {
        let mut row = vec![0; width as usize * 4];
        image.for_each_row(|pixels| {
            for (target, value) in row.as_chunks_mut::<4>().0.iter_mut().zip(pixels) {
                *target = rgba(*value);
            }
            stream.write_all(&row).map_err(|error| error.to_string())
        })
    })
}

/// 【滚动截图】【像素转换】把 value 的 ARGB 转成不透明 RGBA，返回编码通道，不改动 RGB 原值
fn rgba(value: u32) -> [u8; 4] {
    [(value >> 16) as u8, (value >> 8) as u8, value as u8, 255]
}

/// 【滚动截图】【流式容器】按 width、height 创建 PNG 并由 write_rows 提供行，返回编码字节或写入错误
fn encode_rows(
    width: u32,
    height: u32,
    write_rows: impl FnOnce(&mut dyn Write) -> Result<(), String>,
) -> Result<Vec<u8>, String> {
    let row_bytes = width as usize * 4;
    let raw_bytes = (row_bytes + 1)
        .checked_mul(height as usize)
        .ok_or("invalid scroll size")?;
    let maximum = raw_bytes
        .checked_add(raw_bytes.div_ceil(128))
        .and_then(|bytes| bytes.checked_add(STREAM_BYTES))
        .ok_or("invalid PNG size")?;
    let mut output = Vec::new();
    // 1. 【滚动截图】【压缩容量】预留最坏压缩量，避免随机纹理输出增长时同时保留两份完整PNG
    output
        .try_reserve_exact(maximum)
        .map_err(|_| "cannot reserve scroll PNG")?;
    let mut encoder = png::Encoder::new(&mut output, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_deflate_compression(png::DeflateCompression::Level(1));
    encoder.set_filter(png::Filter::Paeth);
    let mut writer = encoder.write_header().map_err(|error| error.to_string())?;
    {
        let mut stream = writer
            .stream_writer_with_size(STREAM_BYTES)
            .map_err(|error| error.to_string())?;
        // 2. 【滚动截图】【像素逐行】保存借用原图，完成复制则消费原块，两条路径使用相同无损编码
        write_rows(&mut stream)?;
        stream.finish().map_err(|error| error.to_string())?;
    }
    writer.finish().map_err(|error| error.to_string())?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【滚动截图测试】【逐行PNG】无参数，双轴编码保持全部像素，并保留可继续捕获的原图及其分配范围，无返回值
    #[test]
    fn row_encoding_preserves_all_pixels_and_the_live_session() {
        for axis in [shot_stitch::Axis::Vertical, shot_stitch::Axis::Horizontal] {
            let pixels = (0..257 * 301)
                .map(|index| 0xff00_0000 | ((index * 7919) & 0x00ff_ffff))
                .collect();
            let original = shot_image::Image::from_pixels(257, 301, pixels).unwrap();
            let mut stitcher = shot_stitch::Stitcher::new(Default::default()).unwrap();
            stitcher.set_axis(axis).unwrap();
            stitcher.push(original.clone()).unwrap();
            let storage = stitcher.pixel_storage();
            let view = stitcher.image_view().unwrap();
            let expected = crate::scroll::source::to_pixmap(view).unwrap();
            let decoded = tiny_skia::Pixmap::decode_png(&encode(view).unwrap()).unwrap();
            assert_eq!(decoded.data(), expected.data(), "axis={axis:?}");
            assert_eq!((decoded.width(), decoded.height()), (257, 301));
            assert_eq!(stitcher.pixel_storage(), storage);
            assert!(stitcher.image_view().is_some());
            let consumed =
                tiny_skia::Pixmap::decode_png(&consume(stitcher.take_image().unwrap()).unwrap())
                    .unwrap();
            assert_eq!(consumed.data(), expected.data(), "axis={axis:?}");
        }
    }

    /// 【滚动截图测试】【高熵压缩】无参数，验证随机纹理 PNG 不膨胀超过预留量且全部像素保持；无返回值
    #[test]
    fn noisy_image_does_not_expand_past_reserved_capacity() {
        let mut value = 42_u32;
        let pixels = (0..512 * 512)
            .map(|_| {
                value ^= value << 13;
                value ^= value >> 17;
                value ^= value << 5;
                value | 0xff00_0000
            })
            .collect();
        let image = shot_image::Image::from_pixels(512, 512, pixels).unwrap();
        let mut stitcher = shot_stitch::Stitcher::new(Default::default()).unwrap();
        stitcher.push(image).unwrap();
        let expected = crate::scroll::source::to_pixmap(stitcher.image_view().unwrap()).unwrap();
        let encoded = consume(stitcher.take_image().unwrap()).unwrap();
        assert!(encoded.len() < 512 * 512 * 4 + STREAM_BYTES);
        assert_eq!(
            tiny_skia::Pixmap::decode_png(&encoded).unwrap().data(),
            expected.data()
        );
    }
}
