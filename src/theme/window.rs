//! 【界面主题】【窗口配色】结果窗口与历史窗口沿用原版设置界面明暗配色

use crate::theme::Rgba;

#[derive(Clone, Copy)]
pub struct Palette {
    pub window: Rgba,
    pub base: Rgba,
    pub error: Rgba,
    pub text: Rgba,
    pub muted: Rgba,
    pub border: Rgba,
    pub alternate: Rgba,
    pub accent: Rgba,
    pub on_accent: Rgba,
    pub pressed: Rgba,
    pub segment: Rgba,
    pub selected_segment: Rgba,
    pub more_icon: Rgba,
}

impl Palette {
    /// 【文字识别】【主题读取】按共享主题亮度返回原版明亮或暗色调色板，无参数；仅 OCR 窗口使用这些颜色
    pub fn current() -> Self {
        Self::new(crate::theme::Mode::current() == crate::theme::Mode::Light)
    }

    /// 【文字识别】【主题生成】按 light 返回原版设置界面调色板及 OCR 独有页签颜色，无其他参数
    pub fn new(light: bool) -> Self {
        if light {
            Self {
                window: Rgba(248, 250, 252, 255),
                base: Rgba(255, 255, 255, 255),
                error: Rgba(185, 28, 28, 255),
                text: Rgba(15, 23, 42, 255),
                muted: Rgba(100, 116, 139, 255),
                border: Rgba(203, 213, 225, 255),
                alternate: Rgba(226, 232, 240, 255),
                accent: Rgba(13, 148, 136, 255),
                on_accent: Rgba(255, 255, 255, 255),
                pressed: Rgba(204, 251, 241, 255),
                segment: Rgba(232, 232, 232, 255),
                selected_segment: Rgba(255, 255, 255, 255),
                more_icon: Rgba(71, 85, 105, 255),
            }
        } else {
            Self {
                window: Rgba(15, 23, 42, 255),
                base: Rgba(15, 23, 42, 255),
                error: Rgba(252, 165, 165, 255),
                text: Rgba(241, 245, 249, 255),
                muted: Rgba(148, 163, 184, 255),
                border: Rgba(51, 65, 85, 255),
                alternate: Rgba(30, 41, 59, 255),
                accent: Rgba(94, 234, 212, 255),
                on_accent: Rgba(15, 23, 42, 255),
                pressed: Rgba(22, 61, 64, 255),
                segment: Rgba(45, 45, 45, 255),
                selected_segment: Rgba(71, 71, 71, 255),
                more_icon: Rgba(203, 213, 225, 255),
            }
        }
    }
}
