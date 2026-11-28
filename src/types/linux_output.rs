//! 【Linux输出】【公共描述】保存两种显示协议共用的几何信息，协议对象单独持有

use smithay_client_toolkit::output::{Mode, OutputInfo};
use wayland_client::protocol::wl_output::{Subpixel, Transform};

#[derive(Debug, Clone)]
pub struct Info {
    pub id: u32,
    pub name: Option<String>,
    pub description: Option<String>,
    pub location: (i32, i32),
    pub logical_position: Option<(i32, i32)>,
    pub logical_size: Option<(i32, i32)>,
    pub physical_size: (i32, i32),
    pub make: String,
    pub model: String,
    pub scale_factor: i32,
    pub subpixel: Subpixel,
    pub transform: Transform,
    pub modes: Vec<Mode>,
}

impl From<OutputInfo> for Info {
    /// 【Linux输出】【Wayland投影】按 info 保留合成器返回的几何与模式，返回协议无关描述
    fn from(info: OutputInfo) -> Self {
        Self {
            id: info.id,
            name: info.name,
            description: info.description,
            location: info.location,
            logical_position: info.logical_position,
            logical_size: info.logical_size,
            physical_size: info.physical_size,
            make: info.make,
            model: info.model,
            scale_factor: info.scale_factor,
            subpixel: info.subpixel,
            transform: info.transform,
            modes: info.modes,
        }
    }
}

impl Info {
    /// 【Linux输出】【逻辑尺寸】优先返回 xdg-output 逻辑尺寸，缺失扩展时按当前物理模式、旋转方向与整数缩放换算
    pub fn logical_size(&self) -> Option<(i32, i32)> {
        if let Some(logical) = self.logical_size {
            return Some(logical);
        }
        let mode = self.modes.iter().find(|mode| mode.current)?;
        let (width, height) = if matches!(
            self.transform,
            Transform::_90 | Transform::_270 | Transform::Flipped90 | Transform::Flipped270
        ) {
            (mode.dimensions.1, mode.dimensions.0)
        } else {
            mode.dimensions
        };
        if width <= 0 || height <= 0 {
            return None;
        }
        let factor = self.scale_factor.max(1);
        Some((
            (width + factor - 1) / factor,
            (height + factor - 1) / factor,
        ))
    }

    /// 【Linux输出】【缩放比例】按较长边比较物理模式与逻辑尺寸计算一百二十分之一精度比例，缺失模式时采用整数缩放因子
    pub fn scale(&self) -> f32 {
        let physical = self
            .modes
            .iter()
            .find(|mode| mode.current)
            .map(|mode| mode.dimensions.0.max(mode.dimensions.1));
        let logical = self.logical_size().map(|(width, height)| width.max(height));
        let (Some(physical), Some(logical)) = (physical, logical) else {
            return (self.scale_factor.max(1) as f32).clamp(0.25, 8.0);
        };
        if physical <= 0 || logical <= 0 {
            return (self.scale_factor.max(1) as f32).clamp(0.25, 8.0);
        }
        let scale = (physical as f32 / logical as f32 * 120.0).round() / 120.0;
        scale.clamp(0.25, 8.0)
    }

    /// 【Linux输出】【X11投影】按 id、name、position、size 构造物理像素坐标描述，返回一倍比例输出
    pub fn x11(id: u32, name: String, position: (i32, i32), size: (i32, i32)) -> Self {
        Self {
            id,
            description: Some(name.clone()),
            name: Some(name),
            location: position,
            logical_position: Some(position),
            logical_size: Some(size),
            physical_size: (0, 0),
            make: String::new(),
            model: String::new(),
            scale_factor: 1,
            subpixel: Subpixel::Unknown,
            transform: Transform::Normal,
            modes: vec![Mode {
                dimensions: size,
                refresh_rate: 0,
                current: true,
                preferred: true,
            }],
        }
    }
}
