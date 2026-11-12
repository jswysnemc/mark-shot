//! 【X11像素】【原生格式】按服务器字节序、颜色掩码与扫描行对齐转换RGBA像素

use super::connection::Display;
use x11rb::{
    connection::{Connection, RequestConnection},
    protocol::xproto::*,
};

pub struct Format {
    bytes: usize,
    pad: usize,
    little: bool,
    masks: [u32; 3],
}

impl Format {
    /// 【X11像素】【格式查询】按display、depth和visual查找真实像素布局，返回不支持格式错误
    pub fn new(display: &Display, depth: u8, visual: Visualid) -> crate::AppResult<Self> {
        let setup = display.connection.setup();
        let format = setup
            .pixmap_formats
            .iter()
            .find(|format| format.depth == depth)
            .ok_or("X11 pixel format missing")?;
        let visual = display
            .screen()
            .allowed_depths
            .iter()
            .flat_map(|depth| &depth.visuals)
            .find(|item| item.visual_id == visual)
            .ok_or("X11 visual missing")?;
        if visual.class != VisualClass::TRUE_COLOR || !matches!(format.bits_per_pixel, 16 | 24 | 32)
        {
            return Err("X11 requires a 16, 24 or 32 bit TrueColor visual".into());
        }
        Ok(Self {
            bytes: usize::from(format.bits_per_pixel / 8),
            pad: usize::from(format.scanline_pad / 8),
            little: setup.image_byte_order == ImageOrder::LSB_FIRST,
            masks: [visual.red_mask, visual.green_mask, visual.blue_mask],
        })
    }

    /// 【X11像素】【扫描行宽度】按width计算协议对齐字节数，返回一行长度
    pub fn stride(&self, width: usize) -> usize {
        (width * self.bytes).div_ceil(self.pad) * self.pad
    }

    /// 【X11像素】【读取转换】按data、width和height解码服务器像素，返回不透明RGBA或长度错误
    pub fn decode(&self, data: &[u8], width: usize, height: usize) -> crate::AppResult<Vec<u8>> {
        let mut rgba = Vec::new();
        self.decode_into(data, width, height, &mut rgba)?;
        Ok(rgba)
    }

    /// 【X11像素】【复用读取】按 data、width、height 解码到 rgba，复用容量，返回格式或长度错误
    pub(crate) fn decode_into(
        &self,
        data: &[u8],
        width: usize,
        height: usize,
        rgba: &mut Vec<u8>,
    ) -> crate::AppResult {
        let stride = self.stride(width);
        if data.len() < stride * height {
            return Err("truncated X11 image".into());
        }
        rgba.resize(width * height * 4, 255);
        if self.bytes == 4 && self.little && self.masks == [0xff0000, 0xff00, 0xff] {
            for (source, target) in data
                .chunks_exact(stride)
                .take(height)
                .zip(rgba.chunks_exact_mut(width * 4))
            {
                for (source, target) in source[..width * 4]
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .zip(target.as_chunks_mut::<4>().0)
                {
                    *target = [source[2], source[1], source[0], 255];
                }
            }
            return Ok(());
        }
        for row in 0..height {
            for column in 0..width {
                let source = &data[row * stride + column * self.bytes..][..self.bytes];
                let mut value = 0u32;
                for (index, byte) in source.iter().enumerate() {
                    value |= u32::from(*byte)
                        << (8 * if self.little {
                            index
                        } else {
                            self.bytes - 1 - index
                        });
                }
                for (channel, mask) in self.masks.iter().copied().enumerate() {
                    let component = (value & mask) >> mask.trailing_zeros();
                    rgba[(row * width + column) * 4 + channel] =
                        (component * 255 / (mask >> mask.trailing_zeros())) as u8;
                }
            }
        }
        Ok(())
    }

    /// 【X11像素】【写入转换】按rgba和size编码服务器扫描行，返回协议缓冲或尺寸错误
    pub fn encode(&self, rgba: &[u8], size: (u16, u16)) -> crate::AppResult<Vec<u8>> {
        let (width, height) = (usize::from(size.0), usize::from(size.1));
        if rgba.len() != width * height * 4 {
            return Err("X11 frame dimensions differ from pixels".into());
        }
        let mut data = Vec::new();
        self.encode_region(rgba, width, (0, 0, width, height), &mut data);
        Ok(data)
    }

    /// 【X11像素】【局部编码】按 rgba、source_width、area 转换指定范围到可复用 data，调用方保证边界有效
    fn encode_region(
        &self,
        rgba: &[u8],
        source_width: usize,
        area: (usize, usize, usize, usize),
        data: &mut Vec<u8>,
    ) {
        let (x, y, width, height) = area;
        let stride = self.stride(width);
        data.resize(stride * height, 0);
        if self.bytes == 4 && self.little && self.masks == [0xff0000, 0xff00, 0xff] {
            for row in 0..height {
                let start = ((y + row) * source_width + x) * 4;
                crate::utils::copy_swizzled(
                    &mut data[row * stride..row * stride + width * 4],
                    &rgba[start..start + width * 4],
                );
            }
            return;
        }
        for row in 0..height {
            for column in 0..width {
                let pixel = &rgba[((y + row) * source_width + x + column) * 4..][..4];
                let mut value = 0u32;
                for (channel, mask) in self.masks.iter().copied().enumerate() {
                    value |= ((u32::from(pixel[channel]) * (mask >> mask.trailing_zeros()) + 127)
                        / 255)
                        << mask.trailing_zeros();
                }
                for index in 0..self.bytes {
                    data[row * stride + column * self.bytes + index] = (value
                        >> (8 * if self.little {
                            index
                        } else {
                            self.bytes - 1 - index
                        }))
                        as u8;
                }
            }
        }
    }
}

/// 【X11像素】【窗口提交】按display、window、gc、size和rgba分块上传，返回协议错误，避免超过最大请求长度
pub fn present(
    display: &Display,
    window: Window,
    gc: Gcontext,
    size: (u16, u16),
    rgba: &[u8],
) -> crate::AppResult {
    present_region(display, window, gc, size, rgba, None)
}

/// 【X11像素】【损坏上传】按 damage 只转换并上传变化区域，rgba 保留完整帧，临时缓冲限制在 256KiB 或单行大小
pub(crate) fn present_region(
    display: &Display,
    window: Window,
    gc: Gcontext,
    size: (u16, u16),
    rgba: &[u8],
    damage: Option<crate::types::DamageRect>,
) -> crate::AppResult {
    let width = usize::from(size.0);
    let height = usize::from(size.1);
    if rgba.len() != width * height * 4 {
        return Err("X11 frame dimensions differ from pixels".into());
    }
    let (x, y, w, h) = damage.unwrap_or((0, 0, size.0.into(), size.1.into()));
    let right = x.saturating_add(w).min(size.0.into()) as usize;
    let bottom = y.saturating_add(h).min(size.1.into()) as usize;
    let (x, y) = (x as usize, y as usize);
    if x >= right || y >= bottom {
        return Ok(());
    }
    let format = Format::new(
        display,
        display.screen().root_depth,
        display.screen().root_visual,
    )?;
    let stride = format.stride(right - x);
    let rows = ((display
        .connection
        .maximum_request_bytes()
        .saturating_sub(64)
        .min(256 * 1024))
        / stride)
        .max(1)
        .min(bottom - y);
    let mut data = Vec::with_capacity(rows * stride);
    let mut requests = Vec::new();
    for start in (y..bottom).step_by(rows) {
        let count = rows.min(bottom - start);
        format.encode_region(rgba, width, (x, start, right - x, count), &mut data);
        requests.push(display.connection.put_image(
            ImageFormat::Z_PIXMAP,
            window,
            gc,
            (right - x) as u16,
            count as u16,
            x as i16,
            start as i16,
            0,
            display.screen().root_depth,
            &data,
        )?);
    }
    // 1. 【X11像素】【顺序屏障】先发送整批请求再查询错误，避免每个扫描块独立等待服务器往返
    display.connection.flush()?;
    for request in requests {
        request.check()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::Format;

    /// 【X11像素测试】【格式往返】无参数，验证两种字节序、扫描行填充及RGB565通道转换
    #[test]
    fn decodes_truecolor_formats_and_padding() {
        for little in [true, false] {
            let format = Format {
                bytes: 3,
                pad: 4,
                little,
                masks: [0xff0000, 0xff00, 0xff],
            };
            let original = [255, 0, 0, 255, 0, 128, 255, 255];
            let encoded = format.encode(&original, (1, 2)).unwrap();
            assert_eq!(encoded.len(), 8);
            assert_eq!(format.decode(&encoded, 1, 2).unwrap(), original);
        }
        let format = Format {
            bytes: 2,
            pad: 4,
            little: true,
            masks: [0xf800, 0x7e0, 0x1f],
        };
        assert_eq!(
            format.decode(&[0, 248, 0, 0], 1, 1).unwrap(),
            [255, 0, 0, 255]
        );
        assert!(format.decode(&[], 1, 1).is_err());
    }
}
