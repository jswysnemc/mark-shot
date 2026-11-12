//! 【扫码】【本地解码】复用图片库并限制解码尺寸，不引入模型、网络或图片格式依赖

use super::Code;
use rxing::multi::MultipleBarcodeReader;
use std::io::Cursor;

pub const INPUT_LIMIT: usize = 64 * 1024 * 1024;
const PIXEL_LIMIT: u64 = 32 * 1024 * 1024;

/// 【扫码】【图像识别】按 bytes 解码图片并识别多个码，返回码制与内容；空结果和损坏图片分别处理
pub fn decode(bytes: &[u8]) -> Result<Vec<Code>, String> {
    if bytes.is_empty() || bytes.len() > INPUT_LIMIT {
        return Err("图片为空或超过 64 MiB / Empty image or input exceeds 64 MiB".into());
    }
    // 1. 【扫码】【输入限制】先读取尺寸，再分配最多 128 MiB 的像素缓存
    let reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|error| error.to_string())?;
    let (width, height) = reader
        .into_dimensions()
        .map_err(|error| error.to_string())?;
    if width == 0 || height == 0 || u64::from(width) * u64::from(height) > PIXEL_LIMIT {
        return Err("图片超过 3200 万像素 / Image exceeds 32 megapixels".into());
    }
    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|error| error.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let rgba = reader
        .decode()
        .map_err(|error| error.to_string())?
        .into_rgba8();
    let mut luma = Vec::with_capacity(width as usize * height as usize);
    // 2. 【扫码】【透明背景】把透明像素合成到白色，避免黑色透明背景吞掉二维码留白
    for pixel in rgba.pixels() {
        let [r, g, b, a] = pixel.0.map(u32::from);
        let gray = (r * 299 + g * 587 + b * 114 + 500) / 1000;
        luma.push(((gray * a + 255 * (255 - a) + 127) / 255) as u8);
    }
    drop(rgba);
    // 3. 【扫码】【独立极性】正常码识别成功也必须扫描反色，复用灰度缓存避免复制整幅图像
    let mut codes = Vec::new();
    for inverted in [false, true] {
        if inverted {
            for pixel in &mut luma {
                *pixel = 255 - *pixel;
            }
        }
        for code in scan_polarity(&luma, width, height)? {
            let value = Code {
                format: code.getBarcodeFormat().to_string(),
                text: code.getText().into(),
            };
            if !codes.contains(&value) {
                codes.push(value);
            }
        }
    }
    Ok(codes)
}

/// 【扫码】【单极性识别】按 luma、width、height 借用灰度像素，返回当前极性的全部结果；无结果返回空列表
fn scan_polarity(luma: &[u8], width: u32, height: u32) -> Result<Vec<rxing::RXingResult>, String> {
    let hints = rxing::DecodeHints {
        TryHarder: Some(true),
        AlsoInverted: Some(false),
        ..Default::default()
    };
    let source = rxing::Luma8Source::new_with_slice(luma, width, height)
        .map_err(|error| error.to_string())?;
    let mut bitmap = rxing::BinaryBitmap::new(rxing::common::HybridBinarizer::new(source));
    let mut reader = rxing::multi::GenericMultipleBarcodeReader::new(
        rxing::MultiUseMultiFormatReader::default(),
    );
    match reader.decode_multiple_with_hints(&mut bitmap, &hints) {
        Ok(codes) => Ok(codes),
        Err(rxing::Exceptions::NotFoundException(_)) => Ok(Vec::new()),
        Err(error) => Err(error.to_string()),
    }
}
