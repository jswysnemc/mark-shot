//! 【Windows剪贴板】【图像格式】透明DIBV5与PNG之间保持原始行序和颜色

/// 【Windows剪贴板】【DIBV5编码】按png返回顶向32位BGRA与透明位掩码，不应用编辑器预乘，不缩放原图
pub(super) fn dib(png: &[u8]) -> crate::AppResult<Vec<u8>> {
    let mut decoder = image::ImageReader::new(std::io::Cursor::new(png)).with_guessed_format()?;
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(128 * 1024 * 1024);
    decoder.limits(limits);
    let image = decoder.decode()?.into_rgba8();
    let (width, height) = image.dimensions();
    if width > i32::MAX as u32 || height > i32::MAX as u32 {
        return Err("invalid clipboard image dimensions".into());
    }
    let mut bytes = vec![0_u8; 124];
    for (offset, value) in [
        (0, 124),
        (4, width),
        (8, (-(height as i32)) as u32),
        (16, 3),
        (20, image.as_raw().len() as u32),
        (40, 0xff0000),
        (44, 0xff00),
        (48, 0xff),
        (52, 0xff000000),
        (56, 0x73524742),
        (108, 4),
    ] {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    bytes[12..14].copy_from_slice(&1_u16.to_le_bytes());
    bytes[14..16].copy_from_slice(&32_u16.to_le_bytes());
    bytes.reserve(image.as_raw().len());
    for pixel in image.as_raw().as_chunks::<4>().0 {
        bytes.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
    }
    Ok(bytes)
}

/// 【Windows剪贴板】【DIB接收】按dib还原BMP文件头并用受限解码器转为PNG，返回原始尺寸图像或格式错误
pub(super) fn png(dib: &[u8]) -> crate::AppResult<Vec<u8>> {
    let header = u32::from_le_bytes(dib.get(..4).ok_or("incomplete DIB header")?.try_into()?);
    if !(40..=124).contains(&header) || dib.len() < header as usize {
        return Err("unsupported DIB header".into());
    }
    let colors = u32::from_le_bytes(dib[32..36].try_into()?);
    let bits = u16::from_le_bytes(dib[14..16].try_into()?);
    let compression = u32::from_le_bytes(dib[16..20].try_into()?);
    let palette = if colors != 0 {
        colors
    } else if bits <= 8 {
        1 << bits
    } else {
        0
    };
    let offset = 14_u32
        .checked_add(header)
        .and_then(|value| value.checked_add(palette.checked_mul(4)?))
        .and_then(|value| {
            value.checked_add(if header == 40 && compression == 3 {
                12
            } else {
                0
            })
        })
        .ok_or("invalid DIB palette")?;
    if offset as usize >= dib.len() + 14 {
        return Err("invalid DIB data offset".into());
    }
    let mut bmp = Vec::with_capacity(dib.len() + 14);
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&((dib.len() + 14) as u32).to_le_bytes());
    bmp.extend_from_slice(&[0; 4]);
    bmp.extend_from_slice(&offset.to_le_bytes());
    bmp.extend_from_slice(dib);
    let mut decoder = image::ImageReader::new(std::io::Cursor::new(bmp));
    decoder.set_format(image::ImageFormat::Bmp);
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(128 * 1024 * 1024);
    decoder.limits(limits);
    let image = decoder.decode()?.into_rgba8();
    Ok(crate::utils::encode_rgba(
        image.as_raw(),
        image.width(),
        image.height(),
    ))
}
