//! 【Windows输出】【捕获几何】全桌面统一使用物理坐标，原生显示器DPI保留在设备描述中

use crate::backend::windows::native::Monitor;

#[derive(Clone, Debug)]
pub struct Mode {
    pub current: bool,
    pub dimensions: (i32, i32),
}

#[derive(Clone, Debug)]
pub struct Info {
    pub name: Option<String>,
    pub description: Option<String>,
    pub location: (i32, i32),
    pub logical_position: Option<(i32, i32)>,
    pub logical_size: Option<(i32, i32)>,
    pub modes: Vec<Mode>,
}

#[derive(Clone, Debug)]
pub struct Output {
    pub native: Monitor,
    pub info: Info,
}

impl Output {
    /// 【Windows输出】【设备投影】按native构造共享捕获描述，返回物理像素布局，避免混合DPI桌面交叠或空隙
    pub fn new(native: Monitor) -> Self {
        let bounds = native.bounds;
        let size = (bounds.width as i32, bounds.height as i32);
        Self {
            info: Info {
                name: Some(native.name.clone()),
                description: Some(native.name.clone()),
                location: (bounds.left, bounds.top),
                logical_position: Some((bounds.left, bounds.top)),
                logical_size: Some(size),
                modes: vec![Mode {
                    current: true,
                    dimensions: size,
                }],
            },
            native,
        }
    }

    /// 【Windows输出】【像素比例】无参数，返回统一物理桌面坐标比例，原图捕获与导出不经过DPI重采样
    pub fn scale(&self) -> f32 {
        1.0
    }
}
