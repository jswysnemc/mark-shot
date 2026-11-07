//! 【界面主题】【菜单配色】贴图和正文右键菜单共用原版明暗令牌

use super::{Mode, Rgba};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    pub accent: Rgba,
    pub background: Rgba,
    pub border: Rgba,
    pub checked_background: Rgba,
    pub danger_hover_background: Rgba,
    pub danger_hover_text: Rgba,
    pub danger_text: Rgba,
    pub disabled_text: Rgba,
    pub hover_background: Rgba,
    pub hover_text: Rgba,
    pub secondary_text: Rgba,
    pub separator: Rgba,
    pub text: Rgba,
}

impl Palette {
    /// 【界面主题】【当前菜单】返回当前进程菜单配色，无参数
    pub fn current() -> Self {
        Self::new(Mode::current())
    }

    /// 【界面主题】【菜单令牌】按 mode 返回原版配色，不修改菜单几何
    pub const fn new(mode: Mode) -> Self {
        match mode {
            Mode::Light => Self {
                accent: Rgba(13, 148, 136, 255),
                background: Rgba(255, 255, 255, 250),
                border: Rgba(15, 23, 42, 26),
                checked_background: Rgba(13, 148, 136, 30),
                danger_hover_background: Rgba(254, 226, 226, 255),
                danger_hover_text: Rgba(185, 28, 28, 255),
                danger_text: Rgba(220, 38, 38, 255),
                disabled_text: Rgba(160, 174, 192, 255),
                hover_background: Rgba(15, 23, 42, 14),
                hover_text: Rgba(15, 23, 42, 255),
                secondary_text: Rgba(100, 116, 139, 255),
                separator: Rgba(15, 23, 42, 20),
                text: Rgba(31, 41, 55, 255),
            },
            Mode::Dark => Self {
                accent: Rgba(94, 234, 212, 255),
                background: Rgba(24, 27, 35, 246),
                border: Rgba(255, 255, 255, 30),
                checked_background: Rgba(45, 212, 191, 36),
                danger_hover_background: Rgba(239, 68, 68, 52),
                danger_hover_text: Rgba(252, 165, 165, 255),
                danger_text: Rgba(248, 113, 113, 255),
                disabled_text: Rgba(100, 116, 139, 255),
                hover_background: Rgba(45, 212, 191, 46),
                hover_text: Rgba(248, 250, 252, 255),
                secondary_text: Rgba(148, 163, 184, 255),
                separator: Rgba(255, 255, 255, 22),
                text: Rgba(229, 231, 235, 255),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// 【主题令牌测试】【原版函数】逐项对照真实 Qt 函数输出的明暗 RGBA，所有角色必须一致，无参数、无返回值
    #[test]
    fn both_palettes_match_the_actual_qt_functions() {
        let reference: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/qt-theme.json")).unwrap();
        for mode in [Mode::Light, Mode::Dark] {
            let palette = Palette::new(mode);
            let expected = &reference[mode.name()]["menu"];
            for (name, color) in [
                ("accent", palette.accent),
                ("background", palette.background),
                ("border", palette.border),
                ("checked_background", palette.checked_background),
                ("danger_hover_background", palette.danger_hover_background),
                ("danger_hover_text", palette.danger_hover_text),
                ("danger_text", palette.danger_text),
                ("disabled_text", palette.disabled_text),
                ("hover_background", palette.hover_background),
                ("hover_text", palette.hover_text),
                ("secondary_text", palette.secondary_text),
                ("separator", palette.separator),
                ("text", palette.text),
            ] {
                assert_eq!(
                    serde_json::json!([color.0, color.1, color.2, color.3]),
                    expected[name],
                    "{}: {name}",
                    mode.name()
                );
            }
        }
    }
}
