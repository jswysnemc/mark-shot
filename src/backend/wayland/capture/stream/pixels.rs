//! 【PipeWire】【缓冲读取】校验实际映射边界、格式与行距，返回紧密排列的 RGBA 帧

use super::PipewireFrame;
use pipewire::spa::param::video::VideoFormat;

/// 【PipeWire】【原始像素】按 bytes、offset、size、stride、dimensions 和 format 复制可见行，返回紧密RGBA缓冲，不读取填充字节
pub(super) fn read(
    bytes: &[u8],
    offset: usize,
    size: usize,
    stride: i32,
    dimensions: (u32, u32),
    format: VideoFormat,
) -> Result<PipewireFrame, String> {
    let (width, height) = dimensions;
    if width == 0
        || height == 0
        || u64::from(width) * u64::from(height) > shot_image::MAX_PIXELS as u64
    {
        return Err("PipeWire frame dimensions are empty or exceed the pixel limit".into());
    }
    let bgr = match format {
        VideoFormat::BGRA | VideoFormat::BGRx => true,
        VideoFormat::RGBA | VideoFormat::RGBx => false,
        other => return Err(format!("PipeWire picked unsupported format {other:?}")),
    };
    let row_bytes = width as usize * 4;
    let stride = if stride == 0 {
        row_bytes as i64
    } else {
        i64::from(stride)
    };
    if stride.unsigned_abs() < row_bytes as u64 {
        return Err("PipeWire row stride is shorter than a visible row".into());
    }
    let span = (height as usize - 1)
        .checked_mul(stride.unsigned_abs() as usize)
        .and_then(|span| span.checked_add(row_bytes))
        .ok_or("PipeWire row span overflow")?;
    if size < span || size > bytes.len() || offset >= bytes.len() {
        return Err("PipeWire chunk is smaller than its visible rows".into());
    }
    let mut pixels = Vec::new();
    pixels
        .try_reserve_exact(row_bytes * height as usize)
        .map_err(|error| format!("PipeWire frame allocation failed: {error}"))?;
    // 1. 【PipeWire】【有界行读取】offset表示首行，负行距从首行向前读取，所有端点按实际映射范围校验
    for row in 0..height {
        let start = offset as i64 + i64::from(row) * stride;
        let start = usize::try_from(start).map_err(|_| "PipeWire negative row origin")?;
        let end = start
            .checked_add(row_bytes)
            .ok_or("PipeWire row endpoint overflow")?;
        pixels.extend_from_slice(
            bytes
                .get(start..end)
                .ok_or("PipeWire visible row exceeds its mapping")?,
        );
    }
    crate::utils::to_rgba(&mut pixels, bgr, false);
    Ok(PipewireFrame {
        pixels,
        width,
        height,
        stride: width * 4,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【PipeWire测试】【行距裁切】正负行距及首行偏移只交付可见像素，不把填充字节带入截图，无参数
    #[test]
    fn padded_rows_offsets_and_negative_stride_keep_exact_pixels() {
        let bytes = [99, 99, 99, 99, 3, 2, 1, 255, 9, 9, 9, 9, 6, 5, 4, 255];
        let forward = read(&bytes, 4, 12, 8, (1, 2), VideoFormat::BGRA).unwrap();
        assert_eq!(forward.pixels, [1, 2, 3, 255, 4, 5, 6, 255]);
        assert_eq!(forward.stride, 4);
        let reverse = read(&bytes, 12, 12, -8, (1, 2), VideoFormat::BGRx).unwrap();
        assert_eq!(reverse.pixels, [4, 5, 6, 255, 1, 2, 3, 255]);
    }

    /// 【PipeWire测试】【无效边界】空图、短行、短映射及不支持格式均明确失败，无参数
    #[test]
    fn malformed_frame_descriptions_never_read_outside_the_mapping() {
        for (offset, size, stride, dimensions, format) in [
            (0, 4, 4, (0, 1), VideoFormat::RGBA),
            (0, 4, 2, (1, 1), VideoFormat::RGBA),
            (3, 4, 4, (1, 1), VideoFormat::RGBA),
            (0, 4, 4, (1, 2), VideoFormat::RGBA),
            (0, 4, -4, (1, 2), VideoFormat::RGBA),
            (0, 4, 4, (1, 1), VideoFormat::RGB),
        ] {
            assert!(read(&[0; 4], offset, size, stride, dimensions, format).is_err());
        }
    }
}
