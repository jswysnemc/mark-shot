//! 【原生窗口】【共用滚动】保留位置、像素位移、角度刻度、来源和修饰键

use crate::keys::Mods;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollSource {
    Wheel,
    Finger,
    Continuous,
    WheelTilt,
    Unknown,
}

#[derive(Debug, Clone, Copy)]
pub struct ScrollInput {
    pub position: (f64, f64),
    pub delta: (f64, f64),
    pub angle: (f64, f64),
    pub source: ScrollSource,
    pub mods: Mods,
}

impl ScrollInput {
    /// 【原生窗口】【旧位移适配】按 position、delta 创建无刻度协议的滚轮信息，供原有内容事件使用，返回明确标记未知来源的输入
    pub fn legacy(position: (f64, f64), delta: (f64, f64)) -> Self {
        Self {
            position,
            delta,
            angle: (-delta.0 * 12.0, -delta.1 * 12.0),
            source: ScrollSource::Unknown,
            mods: Mods::default(),
        }
    }
}
