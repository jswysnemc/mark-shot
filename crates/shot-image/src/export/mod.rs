mod layout;
mod mask;
mod pixels;
mod shadow;

use crate::{Image, ImageError, ImageView, buffer};

pub use layout::{FrameOptions, Layout};

/// 【图像核心】【外框导出】对预乘 source 应用 options，返回物理像素图像；不缩放、不依赖设备 DPR。
pub fn render(source: ImageView<'_>, options: &FrameOptions) -> Result<Image, ImageError> {
    let layout = Layout::new(source.width(), source.height(), options)?;
    if !options.enabled {
        return source.to_image();
    }
    let mut output = buffer::allocate(buffer::pixel_count(layout.width, layout.height)?)?;
    let mask = mask::RoundedMask::new(source.width(), source.height(), options.corner_radius);
    shadow::draw(&mut output, source, &mask, layout, options)?;
    for y in 0..source.height() {
        let offset =
            (y + layout.padding) as usize * layout.width as usize + layout.padding as usize;
        for (x, pixel) in source.row(y).iter().enumerate() {
            output[offset + x] = pixels::over(mask.apply(*pixel, x as i32, y), output[offset + x]);
        }
    }
    Image::from_pixels(layout.width, layout.height, output)
}
