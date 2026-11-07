//! 【界面主题】【进程状态】按需切换主题，不创建定时器或长期工作线程

use super::Rgba;
use crate::config::Theme;
use serde::{Deserialize, Deserializer, Serialize};
use std::sync::{
    OnceLock,
    atomic::{AtomicU8, Ordering},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Light,
    Dark,
}

impl Mode {
    /// 【界面主题】【名称解析】按 value 解析主题，未知值和 system 均使用默认浅色，返回有效模式
    pub fn parse(value: &str) -> Self {
        if value.trim().eq_ignore_ascii_case("dark") {
            Self::Dark
        } else {
            Self::Light
        }
    }

    /// 【界面主题】【亮度识别】按 background 识别旧配色，返回兼容主题；不使用截图像素
    pub fn from_background(background: Rgba) -> Self {
        let maximum = background.0.max(background.1).max(background.2) as u16;
        let minimum = background.0.min(background.1).min(background.2) as u16;
        if maximum + minimum > 255 {
            Self::Light
        } else {
            Self::Dark
        }
    }

    /// 【界面主题】【当前模式】返回进程切换值或启动配置，无参数，不访问磁盘
    pub fn current() -> Self {
        match OVERRIDE.load(Ordering::Relaxed) {
            1 => Self::Light,
            2 => Self::Dark,
            _ => crate::config::get().ui.theme,
        }
    }

    /// 【界面主题】【反向模式】返回当前模式对应的另一主题，无其他参数
    pub fn opposite(self) -> Self {
        match self {
            Self::Light => Self::Dark,
            Self::Dark => Self::Light,
        }
    }

    /// 【界面主题】【配置名称】返回稳定的小写配置值，无其他参数
    pub fn name(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }
}

impl<'de> Deserialize<'de> for Mode {
    /// 【界面主题】【配置反序列化】从 deserializer 读取名称并返回模式，未知名称使用浅色
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(|value| Self::parse(&value))
    }
}

static OVERRIDE: AtomicU8 = AtomicU8::new(0);
static ALTERNATE: OnceLock<Theme> = OnceLock::new();

/// 【界面主题】【当前令牌】返回当前主题，首次切换只准备一套替代颜色并保留原版尺寸，无参数
pub(super) fn theme() -> &'static Theme {
    let config = crate::config::get();
    if Mode::current() == config.ui.theme {
        &config.theme
    } else {
        ALTERNATE.get_or_init(|| crate::config::interface_theme::alternate(config))
    }
}

/// 【界面主题】【继承切换】返回启动后是否执行过切换，无参数；配置测试仍使用各自解析得到的继承值
pub(super) fn overridden() -> bool {
    OVERRIDE.load(Ordering::Relaxed) != 0
}

/// 【界面主题】【切换保存】先保存另一个主题再更新进程状态，返回新模式；保存失败保持当前颜色
pub fn toggle() -> std::io::Result<Mode> {
    let mode = Mode::current().opposite();
    crate::config::interface_theme::save(mode)?;
    apply_saved(mode);
    log::info!("【界面主题】【切换保存】当前主题: {}", mode.name());
    Ok(mode)
}

/// 【界面主题】【外部更新】将 mode 用于本进程，返回是否改变可见主题，不重新保存文件或初始化后台线程
pub(crate) fn apply_saved(mode: Mode) -> bool {
    let changed = Mode::current() != mode;
    OVERRIDE.store(if mode == Mode::Light { 1 } else { 2 }, Ordering::Relaxed);
    changed
}

/// 【界面主题】【快捷键识别】按 chord 使用统一动作配置识别切换快捷键，返回是否匹配
pub fn is_toggle(chord: crate::keys::Chord) -> bool {
    crate::keys::map().lookup(chord) == Some(crate::keys::Action::ToggleTheme)
}
