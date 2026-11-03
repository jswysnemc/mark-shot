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
    /// 【属性取色】【字体就绪】查询 Qt 使用的 Sans Serif 常规与半粗字体；返回独立字库，查询失败保留内置回退
    pub fn new() -> Self {
        let mut system = crate::fonts::bundled();
        let mut family = None;
        // 1. 【属性取色】【定向查询】每种字重只读取匹配文件，不创建常驻进程或后台线程
        for pattern in ["sans-serif", "sans-serif:weight=demibold"] {
            if let Ok(output) = std::process::Command::new("fc-match")
                .args(["-f", "%{file}", pattern])
                .output()
                && output.status.success()
                && let Ok(file) = std::str::from_utf8(&output.stdout)
            {
                let path = std::path::Path::new(file.trim());
                if path.is_file() && system.db_mut().load_font_file(path).is_ok() {
                    family = system
                        .db()
                        .faces()
                        .last()
                        .and_then(|face| face.families.first().map(|item| item.0.clone()));
                }
            }
        }
        if let Some(family) = family {
            system.db_mut().set_sans_serif_family(family);
        }
        Self {
            system,
            swash: SwashCache::new(),
        }
    }
}
