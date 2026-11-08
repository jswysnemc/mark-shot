//! 【选区尺寸】【界面配色】暗色保留原版半透明底板，亮色直接复用工具栏令牌

use crate::theme::{Mode, Rgba, toolbar::Palette};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Colors {
    pub text: Rgba,
    pub muted: Rgba,
    pub hover: Rgba,
    pub fill: Rgba,
    pub border: Rgba,
    pub separator: Rgba,
    pub accent: Rgba,
    pub editor_text: Rgba,
    pub editor_fill: Rgba,
    pub editor_border: Rgba,
    pub editor_selection: Rgba,
}

impl Colors {
    /// 【选区尺寸】【当前配色】按当前主题返回共用字段与弹层颜色，无参数，不改变输入状态
    pub fn current() -> Self {
        Self::new(Mode::current())
    }

    /// 【选区尺寸】【配色映射】按 mode 返回原版颜色，暗色使用独立底板，亮色复用统一工具栏
    pub fn new(mode: Mode) -> Self {
        let palette = Palette::new(mode);
        match mode {
            Mode::System => Self::new(mode.resolved()),
            Mode::Dark => Self {
                text: Rgba(229, 231, 235, 255),
                muted: Rgba(156, 163, 175, 255),
                hover: Rgba(255, 255, 255, 26),
                fill: Rgba(8, 13, 19, 228),
                border: Rgba(255, 255, 255, 30),
                separator: Rgba(255, 255, 255, 36),
                accent: Rgba(94, 234, 212, 255),
                editor_text: Rgba(249, 250, 251, 255),
                editor_fill: Rgba(255, 255, 255, 20),
                editor_border: Rgba(94, 234, 212, 170),
                editor_selection: Rgba(94, 234, 212, 110),
            },
            Mode::Light => Self {
                text: palette.text,
                muted: palette.muted_text,
                hover: palette.control_hover_fill,
                fill: palette.panel_fill,
                border: palette.panel_border,
                separator: palette.separator,
                accent: palette.hover_fill,
                editor_text: palette.strong_text,
                editor_fill: palette.control_fill,
                editor_border: palette.focus_border,
                editor_selection: palette.selection_fill,
            },
        }
    }
}
