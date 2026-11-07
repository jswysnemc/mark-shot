//! 【滚动截图】【面板识别】以实际缓冲中的不透明平色控件识别位置，不向界面添加定位标记

use crate::types::MonitorFrame;

pub type Bounds = (u32, u32, u32, u32);

#[derive(Clone, PartialEq, Eq)]
struct Sample {
    x: u32,
    y: u32,
    rgb: [u8; 3],
}

#[derive(Clone, PartialEq, Eq)]
pub struct Stamp {
    samples: Vec<Sample>,
    anchor: usize,
    source: (u32, u32),
}

impl Stamp {
    /// 【滚动截图】【缓冲采样】从 rgba、size 和 scale 取控件行的稳定平色点，返回足够区分背景的识别样本
    pub fn new(rgba: &[u8], size: (u32, u32), scale: f32) -> Option<Self> {
        if size.0 < 3 || size.1 < 3 || rgba.len() != size.0 as usize * size.1 as usize * 4 {
            return None;
        }
        let mut samples = Vec::new();
        let step = (12.0 * scale).round().max(3.0) as u32;
        let start = size.1.saturating_sub((58.0 * scale).round() as u32).max(1);
        let end = size
            .1
            .saturating_sub((20.0 * scale).round() as u32)
            .max(start);
        // 1. 【滚动截图】【平色过滤】只使用不透明且相邻像素一致的控件点，避开字体边缘与分数缩放插值
        for y in (start..end.min(size.1 - 1)).step_by(step as usize) {
            for x in (1..size.0 - 1).step_by(step as usize) {
                let index = (y as usize * size.0 as usize + x as usize) * 4;
                let pixel = &rgba[index..index + 4];
                if pixel[3] != 255 {
                    continue;
                }
                let same = [-1_i32, 0, 1].into_iter().all(|dy| {
                    [-1_i32, 0, 1].into_iter().all(|dx| {
                        let offset = (((y as i32 + dy) as usize * size.0 as usize)
                            + (x as i32 + dx) as usize)
                            * 4;
                        &rgba[offset..offset + 4] == pixel
                    })
                });
                if same {
                    samples.push(Sample {
                        x,
                        y,
                        rgb: [pixel[0], pixel[1], pixel[2]],
                    });
                }
            }
            // 2. 【滚动截图】【边界采样】保留竖直平直的颜色转换，避免大块同色像素把位置平移多个像素后仍然判为相同
            for x in 2..size.0 - 2 {
                let index = (y as usize * size.0 as usize + x as usize) * 4;
                let current = &rgba[index..index + 4];
                let previous = &rgba[index - 4..index];
                if current[3] != 255 || previous[3] != 255 || current == previous {
                    continue;
                }
                if [-1_i32, 1].into_iter().all(|dy| {
                    let offset = ((y as i32 + dy) as usize * size.0 as usize + x as usize) * 4;
                    &rgba[offset..offset + 4] == current && &rgba[offset - 4..offset] == previous
                }) {
                    samples.push(Sample {
                        x: x - 1,
                        y,
                        rgb: [previous[0], previous[1], previous[2]],
                    });
                    samples.push(Sample {
                        x,
                        y,
                        rgb: [current[0], current[1], current[2]],
                    });
                }
            }
        }
        for x in (2..size.0 - 2).step_by(step as usize) {
            for y in start..end.min(size.1 - 1) {
                let index = (y as usize * size.0 as usize + x as usize) * 4;
                let above = index - size.0 as usize * 4;
                let current = &rgba[index..index + 4];
                let previous = &rgba[above..above + 4];
                if current[3] != 255 || previous[3] != 255 || current == previous {
                    continue;
                }
                if [-1_i32, 1].into_iter().all(|dx| {
                    let offset = (index as isize + dx as isize * 4) as usize;
                    let before = (above as isize + dx as isize * 4) as usize;
                    &rgba[offset..offset + 4] == current && &rgba[before..before + 4] == previous
                }) {
                    samples.push(Sample {
                        x,
                        y: y - 1,
                        rgb: [previous[0], previous[1], previous[2]],
                    });
                    samples.push(Sample {
                        x,
                        y,
                        rgb: [current[0], current[1], current[2]],
                    });
                }
            }
        }
        if samples.len() < 8 {
            return None;
        }
        let anchor = samples
            .iter()
            .enumerate()
            .max_by_key(|(_, sample)| {
                let high = *sample.rgb.iter().max().unwrap();
                let low = *sample.rgb.iter().min().unwrap();
                (high - low, 255 - high)
            })?
            .0;
        if samples
            .iter()
            .all(|sample| sample.rgb == samples[anchor].rgb)
        {
            return None;
        }
        Some(Self {
            samples,
            anchor,
            source: size,
        })
    }

    /// 【滚动截图】【实际定位】在 frame 中按 displayed 与 source 尺寸寻找控件，previous 优先快速复核，返回完整面板边界
    pub fn locate(
        &self,
        frame: &MonitorFrame,
        displayed: (u32, u32),
        source: (u32, u32),
        previous: Option<Bounds>,
    ) -> Option<Bounds> {
        let (width, height, stride) = (frame.pw_width, frame.pw_height, frame.pw_stride as usize);
        let row = (width as usize).checked_mul(4)?;
        if width == 0
            || height == 0
            || stride < row
            || stride
                .checked_mul(height.saturating_sub(1) as usize)?
                .checked_add(row)?
                > frame.pixels.len()
            || displayed.0 == 0
            || displayed.1 == 0
            || source.0 == 0
            || source.1 == 0
        {
            return None;
        }
        if source != self.source {
            return None;
        }
        let point = |sample: &Sample| {
            (
                (sample.x as f64 * displayed.0 as f64 / source.0 as f64).round() as u32,
                (sample.y as f64 * displayed.1 as f64 / source.1 as f64).round() as u32,
                sample.rgb,
            )
        };
        let matches = |x: u32, y: u32| {
            if x.checked_add(displayed.0).is_none_or(|right| right > width)
                || y.checked_add(displayed.1)
                    .is_none_or(|bottom| bottom > height)
            {
                return false;
            }
            self.samples.iter().all(|sample| {
                let (px, py, rgb) = point(sample);
                if px >= displayed.0 || py >= displayed.1 {
                    return false;
                }
                let index = (y + py) as usize * stride + (x + px) as usize * 4;
                (0..3).all(|channel| frame.pixels[index + channel].abs_diff(rgb[channel]) <= 2)
            })
        };
        if let Some((x, y, _, _)) = previous
            && matches(x, y)
        {
            return Some((x, y, displayed.0, displayed.1));
        }
        let (anchor_x, anchor_y, rgb) = point(&self.samples[self.anchor]);
        for y in anchor_y..height {
            for x in anchor_x..width {
                let index = y as usize * stride + x as usize * 4;
                if (0..3).all(|channel| frame.pixels[index + channel].abs_diff(rgb[channel]) <= 2)
                    && matches(x - anchor_x, y - anchor_y)
                {
                    return Some((x - anchor_x, y - anchor_y, displayed.0, displayed.1));
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests;
