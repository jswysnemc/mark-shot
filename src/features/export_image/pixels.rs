//! 【图像导出】【像素转换】连接 tiny-skia 的预乘 RGBA 与图像核心的预乘 ARGB，不改变像素比例

use std::io;

/// 【图像导出】【核心输入】将 source 的预乘 RGBA 转为 ARGB，返回有界图像缓冲或分配错误
pub(super) fn to_argb(source: &tiny_skia::Pixmap) -> io::Result<shot_image::Image> {
    let mut output =
        shot_image::buffer::allocate(source.width() as usize * source.height() as usize)
            .map_err(io::Error::other)?;
    for (pixel, target) in source.data().as_chunks::<4>().0.iter().zip(&mut output) {
        *target = u32::from_be_bytes([pixel[3], pixel[0], pixel[1], pixel[2]]);
    }
    shot_image::Image::from_pixels(source.width() as i32, source.height() as i32, output)
        .map_err(io::Error::other)
}

/// 【图像导出】【编码输入】将 source 的预乘 ARGB 转为 RGBA，返回保持物理尺寸的绘图缓冲
pub(super) fn to_rgba(source: &shot_image::Image) -> io::Result<tiny_skia::Pixmap> {
    let mut output = tiny_skia::Pixmap::new(source.width() as u32, source.height() as u32)
        .ok_or_else(|| io::Error::other("cannot allocate framed image"))?;
    for (pixel, target) in source
        .pixels()
        .iter()
        .zip(output.data_mut().as_chunks_mut::<4>().0)
    {
        let [alpha, red, green, blue] = pixel.to_be_bytes();
        target.copy_from_slice(&[red, green, blue, alpha]);
    }
    Ok(output)
}
