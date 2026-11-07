//! 【区域识别】【像素读取】RGB 通道归一化和 BT.601 整数灰度

use std::borrow::Cow;

pub(in crate::features::region_detection) struct Gray {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}

impl Gray {
    /// 【区域识别】【灰度缓冲】按 width、height 创建零值单通道缓冲，返回图像
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; width * height],
        }
    }

    /// 【区域识别】【灰度读取】按 x、y 返回灰度值，调用方保证位置在图像内
    pub fn at(&self, x: usize, y: usize) -> u8 {
        self.pixels[y * self.width + x]
    }

    /// 【区域识别】【边缘取样】按 x、y 读取灰度值，越出图像时返回零
    pub fn outside_zero(&self, x: i32, y: i32) -> u8 {
        if x < 0 || y < 0 || x as usize >= self.width || y as usize >= self.height {
            0
        } else {
            self.at(x as usize, y as usize)
        }
    }
}

pub(in crate::features::region_detection) struct Rgb<'a> {
    pub width: usize,
    pub height: usize,
    pub pixels: Cow<'a, [u8]>,
    pub area_scaling: bool,
}

impl<'a> Rgb<'a> {
    /// 【区域识别】【颜色归一化】按 width、height 和 rgba 创建只读 RGB 视图，premultiplied 指明预乘格式；返回图像或尺寸错误
    pub fn new(
        width: usize,
        height: usize,
        rgba: &'a [u8],
        premultiplied: bool,
    ) -> Result<Self, &'static str> {
        if width
            .checked_mul(height)
            .and_then(|length| length.checked_mul(4))
            != Some(rgba.len())
        {
            return Err("region image dimensions do not match RGBA pixels");
        }
        let opaque = rgba.as_chunks::<4>().0.iter().all(|pixel| pixel[3] == 255);
        let pixels = if premultiplied && !opaque {
            // 1. 【区域识别】【透明颜色】仅透明预乘图像需要解除预乘，不透明截图直接借用原始像素
            let mut normal = rgba.to_vec();
            for pixel in normal.as_chunks_mut::<4>().0 {
                let alpha = u32::from(pixel[3]);
                for channel in &mut pixel[..3] {
                    *channel = normal_channel(*channel, alpha);
                }
            }
            Cow::Owned(normal)
        } else {
            Cow::Borrowed(rgba)
        };
        Ok(Self {
            width,
            height,
            pixels,
            area_scaling: premultiplied && opaque,
        })
    }

    /// 【区域识别】【颜色取样】按 x、y 返回 RGB 通道，调用方保证像素位于原图内
    pub fn at(&self, x: usize, y: usize) -> [u8; 3] {
        let index = (y * self.width + x) * 4;
        [
            self.pixels[index],
            self.pixels[index + 1],
            self.pixels[index + 2],
        ]
    }

    /// 【区域识别】【灰度转换】无参数，返回按 BT.601 定点权重转换的灰度缓冲
    pub fn grayscale(&self) -> Gray {
        let mut gray = Gray::new(self.width, self.height);
        for (value, pixel) in gray.pixels.iter_mut().zip(self.pixels.as_chunks::<4>().0) {
            *value = ((19595 * u32::from(pixel[0])
                + 38470 * u32::from(pixel[1])
                + 7471 * u32::from(pixel[2]))
                >> 16) as u8;
        }
        gray
    }
}

/// 【区域识别】【通道色差】比较 a、b 的 RGB 通道，返回最大绝对色差，忽略透明度
pub(in crate::features::region_detection) fn difference(a: [u8; 3], b: [u8; 3]) -> u8 {
    a[0].abs_diff(b[0])
        .max(a[1].abs_diff(b[1]))
        .max(a[2].abs_diff(b[2]))
}

/// 【区域识别】【解除预乘】按 value 和 alpha 恢复普通颜色通道，返回与原版整数逆因子相同的八位值
pub(in crate::features::region_detection) fn normal_channel(value: u8, alpha: u32) -> u8 {
    if alpha == 0 {
        0
    } else if alpha == 255 {
        value
    } else {
        ((u32::from(value) * (0x00ff_00ff / alpha) + 0x8000) >> 16).min(255) as u8
    }
}
