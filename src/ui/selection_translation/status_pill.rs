//! 【框选翻译】【进度胶囊】在文字识别与网络翻译等待阶段展示贴合选区边缘的微状态胶囊

use crate::{
    renderer::Painter,
    ui::{
        language::Language,
        micro_badge::{self, BadgeKind},
    },
};
use cosmic_text::FontSystem;
use tiny_skia::Rect;

#[derive(Clone, Debug, PartialEq)]
pub struct StatusPill {
    pub monitor: usize,
    pub rect: Rect,
    pub label: String,
}

impl StatusPill {
    /// 【框选翻译】【胶囊构造】按 recognizing 阶段或 target 语言测量进度胶囊尺寸，返回未定位的胶囊状态
    pub fn new(recognizing: bool, target: Option<&str>, fonts: &mut FontSystem) -> Self {
        let language = Language::current();
        let label = if recognizing {
            if language == Language::Chinese {
                "正在识别文字…".to_owned()
            } else {
                "Recognizing text...".to_owned()
            }
        } else if let Some(target) = target {
            let display = crate::translation::language::display_for(target, language);
            if language == Language::Chinese {
                format!("正在翻译 · {display}")
            } else {
                format!("Translating · {display}")
            }
        } else if language == Language::Chinese {
            "正在翻译…".to_owned()
        } else {
            "Translating...".to_owned()
        };
        let (w, h) = micro_badge::measure(fonts, &label, "", BadgeKind::Busy, 280.0);
        Self {
            monitor: 0,
            rect: Rect::from_xywh(0.0, 0.0, w, h).unwrap(),
            label,
        }
    }
}

/// 【框选翻译】【进度绘制】将 pill 绘制到覆盖层 p，不写入导出原图缓冲，无返回值
pub fn draw(p: &mut Painter<'_>, pill: &StatusPill) {
    micro_badge::draw(p, pill.rect, BadgeKind::Busy, &pill.label, "", 1.0);
}
