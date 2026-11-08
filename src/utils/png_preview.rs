//! 【图像预览】【流式缩略】按行缩小 PNG，不保留完整原图解码缓冲

use std::{
    io::{self, BufRead, Seek},
    sync::atomic::{AtomicBool, Ordering},
};
use tiny_skia::{IntSize, Pixmap};

const PASSES: [(u32, u32, u32, u32); 7] = [
    (0, 0, 8, 8),
    (4, 0, 8, 8),
    (0, 4, 4, 8),
    (2, 0, 4, 4),
    (0, 2, 2, 4),
    (1, 0, 2, 2),
    (0, 1, 1, 2),
];
type Sum = [u64; 5];

/// 【截图历史】【像素累计】将 pixel 的 color 通道加到 sum，按 alpha 预乘；返回通道校验错误
fn accumulate(sum: &mut Sum, pixel: &[u8], color: png::ColorType) -> io::Result<()> {
    let (rgb, alpha) = match color {
        png::ColorType::Rgba => ([pixel[0], pixel[1], pixel[2]], pixel[3]),
        png::ColorType::Rgb => ([pixel[0], pixel[1], pixel[2]], 255),
        png::ColorType::GrayscaleAlpha => ([pixel[0]; 3], pixel[1]),
        png::ColorType::Grayscale => ([pixel[0]; 3], 255),
        png::ColorType::Indexed => return Err(io::Error::other("unexpanded PNG palette")),
    };
    for channel in 0..3 {
        sum[channel] += u64::from(rgb[channel]) * u64::from(alpha);
    }
    sum[3] += u64::from(alpha);
    sum[4] += 1;
    Ok(())
}

/// 【截图历史】【预乘输出】把 source 累计值转换为 target 的 RGBA 像素；每个目标像素按实际样本数取平均
fn finish(source: &[Sum], target: &mut [u8]) {
    for (sum, pixel) in source.iter().zip(target.as_chunks_mut::<4>().0) {
        let count = sum[4].max(1);
        for channel in 0..3 {
            pixel[channel] = ((sum[channel] + count * 255 / 2) / (count * 255)) as u8;
        }
        pixel[3] = ((sum[3] + count / 2) / count) as u8;
    }
}

/// 【截图历史】【预览解码】将 source 的第一帧缩小到 bounds，cancelled 可中止读取；返回带原 alpha 的预乘预览
pub(crate) fn load(
    source: impl BufRead + Seek,
    bounds: (u32, u32),
    cancelled: &AtomicBool,
) -> io::Result<Pixmap> {
    let mut decoder = png::Decoder::new_with_limits(
        source,
        png::Limits {
            bytes: 4 * 1024 * 1024,
        },
    );
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    decoder.set_ignore_text_chunk(true);
    let mut reader = decoder.read_info().map_err(io::Error::other)?;
    let (source_width, source_height) = (reader.info().width, reader.info().height);
    let factor = (bounds.0.max(1) as f64 / source_width as f64)
        .min(bounds.1.max(1) as f64 / source_height as f64)
        .min(1.0);
    let width = (source_width as f64 * factor).round().max(1.0) as u32;
    let height = (source_height as f64 * factor).round().max(1.0) as u32;
    let mut output = vec![0; width as usize * height as usize * 4];
    let (color, depth) = reader.output_color_type();
    if depth != png::BitDepth::Eight {
        return Err(io::Error::other(
            "PNG transformation did not produce eight-bit pixels",
        ));
    }
    let channels = color.samples();
    if reader.info().interlaced {
        // 1. 【截图历史】【隔行预览】按标准七次扫描直接累计到目标预览，不分配原图缓冲
        let mut sums = vec![[0; 5]; width as usize * height as usize];
        for (x0, y0, dx, dy) in PASSES {
            if x0 >= source_width {
                continue;
            }
            for y in (y0..source_height).step_by(dy as usize) {
                if cancelled.load(Ordering::Relaxed) {
                    return Err(io::Error::new(
                        io::ErrorKind::Interrupted,
                        "preview cancelled",
                    ));
                }
                let row = reader
                    .next_row()
                    .map_err(io::Error::other)?
                    .ok_or_else(|| io::Error::other("PNG row is missing"))?;
                let ty = u64::from(y) * u64::from(height) / u64::from(source_height);
                for (x, pixel) in (x0..source_width)
                    .step_by(dx as usize)
                    .zip(row.data().chunks_exact(channels))
                {
                    let tx = u64::from(x) * u64::from(width) / u64::from(source_width);
                    accumulate(&mut sums[(ty * width as u64 + tx) as usize], pixel, color)?;
                }
            }
        }
        finish(&sums, &mut output);
    } else {
        // 2. 【截图历史】【普通预览】仅保留一行目标累计值，长截图不会按原图高度增加内存
        let mut sums = vec![[0; 5]; width as usize];
        let mut target_y = 0;
        for y in 0..source_height {
            if cancelled.load(Ordering::Relaxed) {
                return Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "preview cancelled",
                ));
            }
            let ty = (u64::from(y) * u64::from(height) / u64::from(source_height)) as usize;
            if ty != target_y {
                finish(
                    &sums,
                    &mut output[target_y * width as usize * 4..(target_y + 1) * width as usize * 4],
                );
                sums.fill([0; 5]);
                target_y = ty;
            }
            let row = reader
                .next_row()
                .map_err(io::Error::other)?
                .ok_or_else(|| io::Error::other("PNG row is missing"))?;
            if row.data().len() != source_width as usize * channels {
                return Err(io::Error::other("invalid PNG row length"));
            }
            for (x, pixel) in row.data().chunks_exact(channels).enumerate() {
                let tx = x * width as usize / source_width as usize;
                accumulate(&mut sums[tx], pixel, color)?;
            }
        }
        finish(
            &sums,
            &mut output[target_y * width as usize * 4..(target_y + 1) * width as usize * 4],
        );
    }
    reader.finish().map_err(io::Error::other)?;
    Pixmap::from_vec(output, IntSize::from_wh(width, height).unwrap())
        .ok_or_else(|| io::Error::other("invalid preview pixels"))
}
