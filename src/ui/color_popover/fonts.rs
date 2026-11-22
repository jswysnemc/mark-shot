//! 【属性取色】【原版字体】首次打开时只查询 Sans Serif 对应的两个字形文件，不枚举系统字体

use cosmic_text::{FontSystem, SwashCache};

pub struct PickerFonts {
    pub system: FontSystem,
    pub swash: SwashCache,
}

impl Default for PickerFonts {
    /// 【属性取色】【默认字体】返回按系统 Sans Serif 匹配的独立字库
    fn default() -> Self {
        Self::new()
    }
}

impl PickerFonts {
    /// 【属性取色】【字符命中】按 text、x 返回十二像素半粗字体的字符索引，与原版十一像素文本内边距一致
    pub fn char_index(&mut self, text: &str, x: f32) -> usize {
        let (buffer, _, _) = crate::renderer::text::shape_single_line(
            &mut self.system,
            text,
            12.0,
            cosmic_text::Weight(600),
            cosmic_text::Style::Normal,
        );
        let x = x - 11.0;
        for run in buffer.layout_runs() {
            for glyph in run.glyphs {
                if x < glyph.x + glyph.w / 2.0 {
                    return text[..glyph.start].chars().count();
                }
            }
        }
        text.chars().count()
    }
    /// 【属性取色】【字体就绪】无参数，返回系统首选 Sans Serif 常规与半粗字体，查询失败保留内置回退
    pub fn new() -> Self {
        Self {
            system: crate::fonts::ui_system::new(),
            swash: SwashCache::new(),
        }
    }
}
