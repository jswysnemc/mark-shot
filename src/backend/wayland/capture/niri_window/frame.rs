//! 【niri】【窗口图像】有界 PNG 解码与合成器预乘颜色导入

use std::{fs::File, io::BufReader, path::Path};

use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits};
use tiny_skia::{IntSize, Pixmap};

use super::Result;
use crate::types::Capture;

const MAX_BYTES: u64 = 256 * 1024 * 1024;

/// 【niri】【图像解码】读取 path 的有界 PNG，scale 为已核对的输出比例；返回原生尺寸与预乘像素
pub(super) fn read(path: &Path, scale: f32) -> Result<Capture> {
    let file = File::open(path)?;
    if file.metadata()?.len() > MAX_BYTES {
        return Err("niri window PNG exceeds the capture size limit".into());
    }
    let mut reader = ImageReader::with_format(BufReader::new(file), ImageFormat::Png);
    let mut limits = Limits::default();
    limits.max_image_width = Some(32768);
    limits.max_image_height = Some(32768);
    limits.max_alloc = Some(MAX_BYTES);
    reader.limits(limits);
    let decoder = reader.into_decoder()?;
    let (width, height) = decoder.dimensions();
    if u64::from(width) * u64::from(height) * 4 > MAX_BYTES {
        return Err("niri window RGBA pixels exceed the capture size limit".into());
    }
    let image = DynamicImage::from_decoder(decoder)?.into_rgba8();
    let size = IntSize::from_wh(width, height).ok_or("niri sent an empty window PNG")?;
    let pixels = image.into_raw();
    // 1. 【niri】【颜色语义】niri 的截图编码直接写入预乘渲染结果，不能按普通 PNG 再预乘一次
    if pixels
        .as_chunks::<4>()
        .0
        .iter()
        .any(|pixel| pixel[..3].iter().any(|channel| *channel > pixel[3]))
    {
        return Err("niri window PNG does not contain the expected premultiplied colors".into());
    }
    let pixmap =
        Pixmap::from_vec(pixels, size).ok_or("niri window pixels do not match their dimensions")?;
    Ok(Capture {
        pixmap: pixmap.into(),
        scale,
    })
}
