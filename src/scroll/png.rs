//! 【滚动截图】【逐行编码】直接读取完整原图视图，编码期间只创建一行RGBA和有界压缩缓冲

use shot_stitch::StitchedView;
use std::io::Write;

const STREAM_BYTES: usize = 64 * 1024;

/// 【滚动截图】【完整PNG】按source逐行编码全部原始像素，返回PNG或编码错误，不创建完整RGBA中间图
pub(crate) fn encode(source: StitchedView<'_>) -> Result<Vec<u8>, String> {
    let width = u32::try_from(source.width()).map_err(|_| "invalid scroll width")?;
    let height = u32::try_from(source.height()).map_err(|_| "invalid scroll height")?;
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
    encoder.set_compression(png::Compression::Fast);
    encoder.set_filter(png::Filter::Paeth);
    let mut writer = encoder.write_header().map_err(|error| error.to_string())?;
    {
        let mut stream = writer
            .stream_writer_with_size(STREAM_BYTES)
            .map_err(|error| error.to_string())?;
        let mut row = vec![0; row_bytes];
        // 2. 【滚动截图】【像素逐行】横轴只改变源坐标，透明度保持原有交付规则，不缩放或量化颜色
        for y in 0..source.height() {
            for x in 0..source.width() {
                let value = source.pixel(x, y);
                row[x as usize * 4..x as usize * 4 + 4].copy_from_slice(&[
                    (value >> 16) as u8,
                    (value >> 8) as u8,
                    value as u8,
                    255,
                ]);
            }
            stream.write_all(&row).map_err(|error| error.to_string())?;
        }
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
        }
    }
}
