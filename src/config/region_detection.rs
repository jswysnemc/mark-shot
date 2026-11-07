//! 【区域识别】【独立开关】窗口信息和图像分析分别配置，原版默认均开启

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct Detection {
    pub enabled: bool,
}

impl Default for Detection {
    /// 【区域识别】【默认配置】无参数，返回开启识别的原版默认值
    fn default() -> Self {
        Self { enabled: true }
    }
}
