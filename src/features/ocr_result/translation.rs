//! 【OCR 翻译】【窗口组合】服务任务、分栏绘制和三类输入分别实现，关闭窗口时自动取消所属任务

pub(super) mod controller;
pub(super) mod draw;
pub(super) mod input;
#[cfg(test)]
mod input_method_tests;
pub(super) mod language_menu;
pub(super) mod layout;
mod notice;
mod state;

use crate::{
    backend::window::WindowWake,
    config::Translation as Config,
    ui::text_editor::{EditorStyle, TextEditor},
};
use cosmic_text::FontSystem;
use std::{path::PathBuf, time::Instant};

pub(super) struct Translation {
    pub ui_language: crate::ui::language::Language,
    pub editor: TextEditor,
    pub language: TextEditor,
    pub state: state::State,
    pub config: Config,
    pub config_path: Option<PathBuf>,
    pub task: Option<crate::translation::task::Task>,
    pub retired: Vec<crate::translation::task::Task>,
    pub retire_deadline: Option<Instant>,
    pub wake: Option<WindowWake>,
    pub language_menu: Option<language_menu::Menu>,
    pub language_offset: f32,
    pub ratio: f32,
    pub splitter_drag: bool,
    pub layout_dirty: bool,
    pub saved_target: String,
    pub characters: usize,
    pub animation_start: Option<Instant>,
    pub animation_deadline: Option<Instant>,
    pub animation_phase: f32,
    pub versions: [(u64, u64); 3],
}

/// 【OCR 翻译】【编辑样式】按 language 返回原版正文或单行输入样式，不修改共享主题或其他编辑器
pub(super) fn style(language: bool) -> EditorStyle {
    let colors = super::palette::Palette::current();
    EditorStyle {
        size: if language { 13.0 } else { 15.0 },
        line_height: if language { 18.0 } else { 20.0 },
        colors: Some([colors.text, colors.text, colors.accent, colors.on_accent]),
    }
}

impl Translation {
    /// 【OCR 翻译】【窗口初始化】按 fonts 和 config 创建互相独立的译文及语言编辑器，返回状态，不启动线程或读取环境凭据
    #[cfg(test)]
    pub fn new(fonts: &mut FontSystem, config: Config) -> Self {
        Self::localized(fonts, config, crate::ui::language::Language::English)
    }

    /// 【OCR 翻译】【语言初始化】按 fonts、config 和 ui_language 创建独立编辑器，预设语言显示本地化标签，持久值保持服务稳定名称
    pub fn localized(
        fonts: &mut FontSystem,
        config: Config,
        ui_language: crate::ui::language::Language,
    ) -> Self {
        let saved_target = config.target_language.clone();
        let displayed = crate::translation::language::display_for(&saved_target, ui_language);
        crate::fonts::ocr_ui::ensure_text(fonts, displayed);
        let mut language = TextEditor::new_with_style(displayed, fonts, style(true));
        language.single_line(fonts);
        language.set_byte_selection(fonts, displayed.len(), displayed.len());
        Self {
            ui_language,
            editor: TextEditor::new_with_style("", fonts, style(false)),
            language,
            state: state::State::default(),
            config,
            config_path: None,
            task: None,
            retired: Vec::new(),
            retire_deadline: None,
            wake: None,
            language_menu: None,
            language_offset: 0.0,
            ratio: 0.6,
            splitter_drag: false,
            layout_dirty: true,
            saved_target,
            characters: 0,
            animation_start: None,
            animation_deadline: None,
            animation_phase: 0.0,
            versions: [(0, 0); 3],
        }
    }

    /// 【OCR 翻译】【当前目标】返回确认输入的稳定值，空输入沿用已保存目标；候选不进入请求或新鲜度比较
    pub fn target(&self) -> String {
        let target =
            crate::translation::language::value_for(&self.language.text(), self.ui_language);
        if target.is_empty() {
            self.saved_target.clone()
        } else {
            target
        }
    }
}

#[cfg(test)]
mod localization_tests {
    use super::*;

    /// 【OCR 翻译测试】【目标隔离】中文界面展示原版预设标签，服务仍使用稳定英文名称，自由目标及译文保持原样，无参数、无返回值
    #[test]
    fn localized_target_labels_do_not_change_service_values_or_translation_text() {
        let mut fonts = crate::fonts::ocr_ui::new();
        let config = Config {
            target_language: "English".into(),
            ..Config::default()
        };
        let mut translation =
            Translation::localized(&mut fonts, config, crate::ui::language::Language::Chinese);
        assert_eq!(translation.language.text(), "英语");
        assert_eq!(translation.target(), "English");
        for value in ["日语", "Copy %1", "任意目标"] {
            translation.language = TextEditor::new_with_style(value, &mut fonts, style(true));
            let expected = if value == "日语" { "Japanese" } else { value };
            assert_eq!(translation.target(), expected);
        }
        translation.editor = TextEditor::new_with_style("Copy %1", &mut fonts, style(false));
        assert_eq!(translation.editor.text(), "Copy %1");
    }
}
