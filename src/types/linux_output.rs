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
