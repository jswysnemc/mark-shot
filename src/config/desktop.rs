//! 【桌面集成】【配置模型】全局快捷键与托盘独立开关，不改变编辑器内部绑定

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct Desktop {
    pub autostart: bool,
    pub tray_enabled: bool,
    pub shortcuts_enabled: bool,
    pub screenshot_shortcut: String,
    pub active_window_shortcut: String,
}

impl Default for Desktop {
    /// 【桌面集成】【默认配置】无参数，Windows 默认启用托盘和截图按键，其他平台保留显式启用方式
    fn default() -> Self {
        Self {
            autostart: false,
            tray_enabled: cfg!(target_os = "windows"),
            shortcuts_enabled: cfg!(target_os = "windows"),
            screenshot_shortcut: "PrintScreen".into(),
            active_window_shortcut: "Alt+PrintScreen".into(),
        }
    }
}

impl Desktop {
    /// 【桌面集成】【服务需求】无参数，返回是否需要常驻注册
    pub fn enabled(&self) -> bool {
        self.tray_enabled || self.shortcuts_enabled
    }
}
