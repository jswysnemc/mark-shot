//! 【界面主题】【继承角色】组件显式颜色与主题继承值分开保存，切换不覆盖组件自定义值

use super::Rgba;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Role {
    Background,
    Foreground,
    Muted,
    Hover,
    Accent,
    OnAccent,
}

impl Role {
    /// 【界面主题】【继承取值】按 self 返回当前主题角色；未切换时返回 fallback，支持独立配置解析
    pub fn resolve(self, fallback: Rgba) -> Rgba {
        if !super::mode::overridden() {
            return fallback;
        }
        let theme = super::mode::theme();
        match self {
            Self::Background => theme.background,
            Self::Foreground => theme.foreground,
            Self::Muted => theme.muted,
            Self::Hover => theme.hover,
            Self::Accent => theme.accent,
            Self::OnAccent => theme.on_accent,
        }
    }
}
