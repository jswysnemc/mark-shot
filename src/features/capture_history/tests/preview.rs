//! 【截图历史测试】【流式像素】隔行 PNG、调色板和灰度预览

use super::super::preview;
use std::{io::Write, sync::atomic::AtomicBool};

/// 【截图历史测试】【预览文件】把 png 写入私有临时文件并加载 bounds 预览；返回预乘图像
fn load(png: &[u8], bounds: (u32, u32)) -> tiny_skia::Pixmap {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(png).unwrap();
    preview::load(file.reopen().unwrap(), bounds, &AtomicBool::new(false)).unwrap()
}

/// 【截图历史测试】【标准 PNG 块】将 kind、data 与 CRC 写入 output，用于独立隔行测试图
fn chunk(output: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    output.extend_from_slice(&(data.len() as u32).to_be_bytes());
    output.extend_from_slice(kind);
    output.extend_from_slice(data);
    let mut crc = crc32fast::Hasher::new();
    crc.update(kind);
    crc.update(data);
    output.extend_from_slice(&crc.finalize().to_be_bytes());
}

/// 【截图历史测试】【独立隔行编码】按标准 Adam7 顺序编码 width、height 的 rgba；使用无压缩 DEFLATE，不依赖预览代码
fn interlaced(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let mut rows = Vec::new();
    for (x0, y0, dx, dy) in [
        (0, 0, 8, 8),
        (4, 0, 8, 8),
        (0, 4, 4, 8),
        (2, 0, 4, 4),
        (0, 2, 2, 4),
        (1, 0, 2, 2),
        (0, 1, 1, 2),
    ] {
        if x0 >= width {
            continue;
        }
        for y in (y0..height).step_by(dy) {
            rows.push(0);
            for x in (x0..width).step_by(dx) {
                let index = (y * width + x) as usize * 4;
                rows.extend_from_slice(&rgba[index..index + 4]);
            }
        }
    }
    assert!(rows.len() < u16::MAX as usize);
    let mut data = vec![0x78, 0x01, 0x01];
    let length = rows.len() as u16;
    data.extend_from_slice(&length.to_le_bytes());
    data.extend_from_slice(&(!length).to_le_bytes());
    data.extend_from_slice(&rows);
    let (mut a, mut b) = (1_u32, 0_u32);
    for value in rows {
        a = (a + u32::from(value)) % 65521;
        b = (b + a) % 65521;
    }
    data.extend_from_slice(&((b << 16) | a).to_be_bytes());
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut header = Vec::new();
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 1]);
    chunk(&mut png, b"IHDR", &header);
    chunk(&mut png, b"IDAT", &data);
    chunk(&mut png, b"IEND", &[]);
    png
}

/// 【截图历史测试】【隔行原像素】独立 Adam7 图在不缩小时恢复全部像素；缩小结果与非隔行图一致
#[test]
fn adam7_preview_matches_independent_png_decode() {
    let (width, height) = (17, 13);
    let rgba: Vec<u8> = (0..width * height)
        .flat_map(|index| {
            [
                ((index * 17) % 256) as u8,
                ((index * 31) % 256) as u8,
                ((index * 7) % 256) as u8,
                [0, 128, 255][index as usize % 3],
            ]
        })
        .collect();
    let png = interlaced(width, height, &rgba);
    assert_eq!(
        image::load_from_memory(&png).unwrap().into_rgba8().as_raw(),
        &rgba
    );
    let preview = load(&png, (width, height));
    let mut expected = rgba.clone();
    crate::utils::premultiply(&mut expected);
    assert_eq!(preview.data(), expected);
    let mut plain = Vec::new();
    let mut encoder = png::Encoder::new(&mut plain, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&rgba)
        .unwrap();
    assert_eq!(load(&png, (3, 2)).data(), load(&plain, (3, 2)).data());
}

/// 【截图历史测试】【颜色格式】灰度透明和调色板输入扩展到预乘 RGBA，保留原透明度
#[test]
fn palette_and_grayscale_alpha_are_supported() {
    for (color, samples) in [
        (
            png::ColorType::GrayscaleAlpha,
            vec![100, 0, 50, 128, 80, 255],
        ),
        (png::ColorType::Indexed, vec![0, 1, 2]),
    ] {
        let mut png = Vec::new();
        let mut encoder = png::Encoder::new(&mut png, 3, 1);
        encoder.set_color(color);
        encoder.set_depth(png::BitDepth::Eight);
        if color == png::ColorType::Indexed {
            encoder.set_palette(vec![100, 0, 50, 50, 100, 150, 80, 60, 40]);
            encoder.set_trns(vec![0, 128, 255]);
        }
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&samples)
            .unwrap();
        let mut expected = image::load_from_memory(&png)
            .unwrap()
            .into_rgba8()
            .into_raw();
        crate::utils::premultiply(&mut expected);
        assert_eq!(load(&png, (3, 1)).data(), expected);
    }
}
