//! 【桌面集成】【配置模型】全局快捷键与托盘独立开关，不改变编辑器内部绑定

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct Desktop {
    pub tray_enabled: bool,
    pub shortcuts_enabled: bool,
    pub screenshot_shortcut: String,
}

impl Default for Desktop {
    /// 【桌面集成】【默认配置】无参数，返回需要用户启用的桌面功能和推荐截图按键
    fn default() -> Self {
        Self {
            tray_enabled: false,
            shortcuts_enabled: false,
            screenshot_shortcut: "PrintScreen".into(),
        }
    }
}

impl Desktop {
    /// 【桌面集成】【服务需求】无参数，返回是否需要常驻注册
    pub fn enabled(&self) -> bool {
        self.tray_enabled || self.shortcuts_enabled
    }
}
