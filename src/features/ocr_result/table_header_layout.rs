//! 【OCR 表格】【表头几何】单独组合标题、行列统计和复制入口，正文网格保持独立

use super::titlebar_layout::rect;
use crate::ui::language::Language;
use tiny_skia::Rect;

#[cfg(test)]
mod tests;

pub(super) struct Header {
    pub title: Rect,
    pub summary: Rect,
}

impl Header {
    /// 【OCR 表格】【表头排列】按 copy、language 返回标题和统计矩形，copy 属于同一标题行，统计只占剩余区域
    pub fn new(copy: Rect, language: Language) -> Self {
        // 1. 【OCR 表格】【标题宽度】沿用原版固定字号的实际控件宽度，中文不能占用英文的留白
        let width = if language == Language::Chinese {
            26.0
        } else {
            34.0
        };
        let title = rect(8.0, copy.top(), width, copy.height());
        // 2. 【OCR 表格】【统计留白】标题和复制入口各保留原版六像素间隔，失败反馈只缩短统计区域
        let left = title.right() + 6.0;
        Self {
            title,
            summary: rect(left, copy.top(), copy.left() - 6.0 - left, copy.height()),
        }
    }
}
