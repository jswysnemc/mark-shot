//! 【图片剪贴板】【兼容格式】按需生成标准 BMP，虚拟机与旧客户端不必支持 PNG 解码

use std::{
    fs::File,
    io::{self, BufReader, Seek, SeekFrom},
};

pub(super) const PNG: &str = "image/png";
pub(super) const BMP: &str = "image/bmp";
pub(super) const X_BMP: &str = "image/x-bmp";
pub(super) const TYPES: [&str; 3] = [PNG, BMP, X_BMP];

/// 【图片剪贴板】【位图编码】从 source 的 PNG 生成匿名 BMP 文件，返回文件；图像内存有界，透明区域合成到白底
pub(super) fn bitmap(source: &File) -> io::Result<File> {
    let mut input = source.try_clone()?;
    input.seek(SeekFrom::Start(0))?;
    let mut reader =
        image::ImageReader::with_format(BufReader::new(input), image::ImageFormat::Png);
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(256 * 1024 * 1024);
    reader.limits(limits);
    let rgba = reader.decode().map_err(io::Error::other)?.into_rgba8();
    let mut rgb = image::RgbImage::new(rgba.width(), rgba.height());
    // 1. 【图片剪贴板】【标准位图】使用二十四位 BGR 位图，透明像素按可见白底合成，避免客户端不支持透明 BMP
    for (target, pixel) in rgb.pixels_mut().zip(rgba.pixels()) {
        let alpha = u32::from(pixel[3]);
        for channel in 0..3 {
            target[channel] =
                ((u32::from(pixel[channel]) * alpha + 255 * (255 - alpha) + 127) / 255) as u8;
        }
    }
    let mut output = tempfile::tempfile()?;
    image::codecs::bmp::BmpEncoder::new(&mut output)
        .encode(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            image::ExtendedColorType::Rgb8,
        )
        .map_err(io::Error::other)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// 【剪贴板格式测试】【位图兼容】无参数，验证标准 BMP 保留尺寸和不透明颜色，透明区域变成白底，无返回值
    #[test]
    fn bmp_roundtrip_keeps_original_pixels_and_flattens_transparency() {
        let image = image::RgbaImage::from_raw(2, 1, vec![13, 45, 79, 255, 0, 0, 0, 0]).unwrap();
        let mut png = Cursor::new(Vec::new());
        image.write_to(&mut png, image::ImageFormat::Png).unwrap();
        let mut source = tempfile::tempfile().unwrap();
        std::io::Write::write_all(&mut source, png.get_ref()).unwrap();
        let mut output = bitmap(&source).unwrap();
        output.seek(SeekFrom::Start(0)).unwrap();
        let decoded =
            image::ImageReader::with_format(BufReader::new(output), image::ImageFormat::Bmp)
                .decode()
                .unwrap()
                .into_rgb8();
        assert_eq!(decoded.dimensions(), (2, 1));
        assert_eq!(decoded.into_raw(), [13, 45, 79, 255, 255, 255]);
    }
}
