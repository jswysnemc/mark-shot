//! 【文本编辑】【滚动范围】按换行后的实际高度同步滚动条和正文视区

use cosmic_text::{Edit, FontSystem};
use tiny_skia::Rect;

use super::TextEditor;
use crate::ui::scrollbar::{self, Geometry};

#[derive(Default)]
pub(super) struct LayoutCache {
    revision: u64,
    width: f32,
    heights: Vec<f32>,
    total: f32,
}

impl TextEditor {
    /// 【文本编辑】【滚动几何】按 fonts 排版并返回 viewport 的滚动条，正文不足一屏返回空值，不改变光标和选区
    pub(crate) fn scrollbar(&mut self, fonts: &mut FontSystem, viewport: Rect) -> Option<Geometry> {
        self.measure_scroll(fonts);
        let offset = self.editor.with_buffer(|buffer| {
            let scroll = buffer.scroll();
            self.scroll_layout
                .heights
                .iter()
                .take(scroll.line)
                .sum::<f32>()
                + scroll.vertical
        });
        scrollbar::geometry(
            viewport,
            self.scroll_layout.total.ceil() as usize,
            self.size.1.floor() as usize,
            offset.round().max(0.0) as usize,
        )
    }

    /// 【文本编辑】【高度缓存】按 fonts 更新实际折行高度，正文版本与宽度不变时复用，返回值为空
    fn measure_scroll(&mut self, fonts: &mut FontSystem) {
        let cache = &mut self.scroll_layout;
        if !cache.heights.is_empty()
            && cache.revision == self.input.revision
            && cache.width == self.size.0
        {
            return;
        }
        self.editor.with_buffer_mut(|buffer| {
            let line_height = buffer.metrics().line_height;
            cache.heights = (0..buffer.lines.len())
                .map(|index| {
                    buffer
                        .line_layout(fonts, index)
                        .map_or(line_height, |lines| {
                            lines
                                .iter()
                                .map(|line| line.line_height_opt.unwrap_or(line_height))
                                .sum::<f32>()
                                .max(line_height)
                        })
                })
                .collect();
        });
        cache.total = cache.heights.iter().sum();
        cache.revision = self.input.revision;
        cache.width = self.size.0;
    }

    /// 【文本编辑】【绝对滚动】将 fonts 排版后的视区移到 offset 像素，自动夹紧端点，不改变文本、光标或选择
    pub(crate) fn scroll_to(&mut self, fonts: &mut FontSystem, offset: usize) {
        self.measure_scroll(fonts);
        let mut remaining = (offset as f32).min((self.scroll_layout.total - self.size.1).max(0.0));
        let mut line = 0;
        for height in &self.scroll_layout.heights {
            if remaining < *height {
                break;
            }
            remaining -= height;
            line += 1;
        }
        self.editor.with_buffer_mut(|buffer| {
            let mut scroll = buffer.scroll();
            scroll.line = line.min(buffer.lines.len().saturating_sub(1));
            scroll.vertical = remaining;
            buffer.set_scroll(scroll);
            buffer.shape_until_scroll(fonts, false);
        });
    }
}
