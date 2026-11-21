//! 【RTF解析】【字体资源】按索引识别等宽与衬线字体

use super::super::html_style;

/// 【RTF解析】【字体信息】记录字体是否等宽以及非等宽时的衬线标记
#[derive(Clone, Copy)]
pub(super) struct FontInfo {
    pub(super) code: bool,
    pub(super) serif: bool,
}

/// 【RTF解析】【字体索引上限】限制恶意 `\fN` 造成的表格扩容
const MAX_FONT_INDEX: i32 = 4096;

/// 【RTF解析】【字体表】按字体索引保存从 `fonttbl` 解析出的字体信息
#[derive(Default)]
pub(super) struct FontTable {
    entries: Vec<Option<FontInfo>>,
}

impl FontTable {
    /// 【RTF解析】【新建字体表】无参数，返回空字体表
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// 【RTF解析】【登记字体】按 index 与 name 识别等宽或衬线字体，越界索引自动扩容
    pub(super) fn define(&mut self, index: i32, name: &str) {
        if !(0..=MAX_FONT_INDEX).contains(&index) {
            return;
        }
        let slot = index as usize;
        if self.entries.len() <= slot {
            self.entries.resize(slot + 1, None);
        }
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        let code = html_style::monospace(name);
        let serif = !code && html_style::serif(name);
        self.entries[slot] = Some(FontInfo { code, serif });
    }

    /// 【RTF解析】【查询字体】按 index 返回字体信息，缺失或非法索引返回空值
    pub(super) fn get(&self, index: i32) -> Option<FontInfo> {
        if index < 0 {
            return None;
        }
        self.entries.get(index as usize).copied().flatten()
    }
}
