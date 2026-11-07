use crate::{ImageError, buffer::pixel_count};

#[derive(Clone, Copy, Debug)]
pub struct FrameOptions {
    pub enabled: bool,
    pub padding: i32,
    pub corner_radius: f64,
    pub shadow_radius: i32,
    pub shadow_offset_y: i32,
    pub shadow_opacity: f64,
    pub shadow_color: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout {
    pub width: i32,
    pub height: i32,
    pub padding: i32,
}

impl Layout {
    /// 根据物理像素 width/height 和 options 计算布局，返回检查过的尺寸或参数错误。
    pub fn new(width: i32, height: i32, options: &FrameOptions) -> Result<Self, ImageError> {
        pixel_count(width, height)?;
        if options.enabled
            && (!options.corner_radius.is_finite()
                || !options.shadow_opacity.is_finite()
                || options.shadow_radius > 4096)
        {
            return Err(ImageError::Parameter);
        }
        let padding = if options.enabled {
            options.padding.max(0)
        } else {
            0
        };
        let border = padding.checked_mul(2).ok_or(ImageError::Dimensions)?;
        let width = width.checked_add(border).ok_or(ImageError::Dimensions)?;
        let height = height.checked_add(border).ok_or(ImageError::Dimensions)?;
        pixel_count(width, height)?;
        Ok(Self {
            width,
            height,
            padding,
        })
    }
}
