//! 【窗口输入法】【有限上下文】周边正文最多四千字节，光标矩形直接复用实际显示编辑器

use super::super::TextEditor;
use crate::{
    features::text::input_method::surrounding,
    types::text_input::{ChangeCause, ContentType, Focus, Snapshot, Surrounding},
};
use cosmic_text::{Cursor, Edit as _, FontSystem, Selection};
use tiny_skia::Rect;

pub(in crate::ui::text_editor) struct Cache {
    revision: u64,
    position: (Cursor, Selection),
    surrounding: Option<Surrounding>,
}

pub struct InputArea {
    pub rect: Rect,
    pub offset: f32,
}

/// 【窗口输入法】【文本光标】按 text 和 offset 返回 UTF-8 字节对应的行位置，非法边界返回空值
pub(super) fn cursor_in_text(text: &str, offset: usize) -> Option<Cursor> {
    crate::features::text::input_method::positions::editor_cursor(text, offset)
}

impl TextEditor {
    /// 【窗口输入法】【文档偏移】返回 cursor 对应的完整文档字节，保留缓冲行结束符，非法位置返回空值
    pub(super) fn byte_offset(&self, cursor: Cursor) -> Option<usize> {
        self.editor.with_buffer(|buffer| {
            let line = buffer.lines.get(cursor.line)?;
            if !line.text().is_char_boundary(cursor.index) {
                return None;
            }
            Some(
                buffer.lines[..cursor.line]
                    .iter()
                    .map(|line| line.text().len() + line.ending().as_str().len())
                    .sum::<usize>()
                    + cursor.index,
            )
        })
    }

    /// 【窗口输入法】【正文位置】返回已确认正文的光标、锚点字节，无参数；候选光标不作为业务位置
    pub fn byte_selection(&self) -> (usize, usize) {
        let cursor = self.editor.cursor();
        let anchor = match self.editor.selection() {
            Selection::None => cursor,
            Selection::Normal(anchor) | Selection::Word(anchor) | Selection::Line(anchor) => anchor,
        };
        (
            self.byte_offset(cursor).unwrap_or(0),
            self.byte_offset(anchor).unwrap_or(0),
        )
    }

    /// 【窗口输入法】【选区定位】用 fonts 设置 cursor 和 anchor 文档字节，返回是否有效；不修改正文或历史
    pub fn set_byte_selection(
        &mut self,
        fonts: &mut FontSystem,
        cursor: usize,
        anchor: usize,
    ) -> bool {
        let text = self.text();
        let (Some(cursor), Some(anchor)) =
            (cursor_in_text(&text, cursor), cursor_in_text(&text, anchor))
        else {
            return false;
        };
        self.interrupt_input();
        self.editor.set_cursor(cursor);
        self.editor.set_selection(if cursor == anchor {
            Selection::None
        } else {
            Selection::Normal(anchor)
        });
        self.editor.shape_as_needed(fonts, false);
        true
    }

    /// 【窗口输入法】【周边范围】从已确认缓冲生成有界上下文，返回窗口；同段落不复制全文，过大的选择返回空值，无参数
    fn surrounding(&self) -> Option<Surrounding> {
        let (cursor, anchor) = self.byte_selection();
        let position = self.editor.cursor();
        let selected = match self.editor.selection() {
            Selection::None => position,
            Selection::Normal(anchor) | Selection::Word(anchor) | Selection::Line(anchor) => anchor,
        };
        if !self.input.single_line && position.line == selected.line {
            return self.editor.with_buffer(|buffer| {
                let line = buffer.lines.get(position.line)?;
                let mut value = surrounding::window(line.text(), position.index, selected.index)?;
                value.document_start += cursor - position.index;
                Some(value)
            });
        }
        let low = cursor.min(anchor);
        let high = cursor.max(anchor);
        if high - low > surrounding::MAX_BYTES {
            return None;
        }
        self.editor.with_buffer(|buffer| {
            let total = buffer
                .lines
                .iter()
                .map(|line| line.text().len() + line.ending().as_str().len())
                .sum::<usize>();
            let allowance = surrounding::MAX_BYTES - (high - low);
            let mut start = low.saturating_sub(allowance / 2);
            let end = total.min(start + surrounding::MAX_BYTES);
            start = start.min(end.saturating_sub(surrounding::MAX_BYTES));
            let mut offset = 0;
            let mut text = String::with_capacity(end - start);
            let mut actual_start = start;
            for line in &buffer.lines {
                for part in [line.text(), line.ending().as_str()] {
                    let part_end = offset + part.len();
                    if offset < end && part_end > start {
                        let mut left = start.saturating_sub(offset);
                        let mut right = (end - offset).min(part.len());
                        while !part.is_char_boundary(left) {
                            left += 1;
                        }
                        while !part.is_char_boundary(right) {
                            right -= 1;
                        }
                        if text.is_empty() {
                            actual_start = offset + left;
                        }
                        text.push_str(&part[left..right]);
                    }
                    offset = part_end;
                }
            }
            if text.contains('\0') {
                return None;
            }
            Some(Surrounding {
                text,
                cursor: i32::try_from(cursor - actual_start).ok()?,
                anchor: i32::try_from(anchor - actual_start).ok()?,
                document_start: actual_start,
            })
        })
    }

    /// 【窗口输入法】【原生快照】用 fonts 和 area 的矩形、偏移定位显示光标，focus、target_id、previous、cause 控制协议；无变化不分配快照
    pub fn input_snapshot(
        &mut self,
        fonts: &mut FontSystem,
        area: InputArea,
        focus: Focus,
        target_id: u64,
        previous: Option<&Snapshot>,
        cause: ChangeCause,
    ) -> Option<Snapshot> {
        let InputArea { rect, offset } = area;
        self.resize(fonts, rect.width(), rect.height());
        let position = (self.editor.cursor(), self.editor.selection());
        if self
            .input
            .context
            .as_ref()
            .is_none_or(|cache| cache.revision != self.input.revision || cache.position != position)
        {
            self.input.context = Some(Cache {
                revision: self.input.revision,
                position,
                surrounding: self.surrounding(),
            });
        }
        let editor = self.display_editor(fonts);
        let cursor = editor.cursor();
        let cursor_rect = editor.with_buffer(|buffer| {
            buffer.layout_runs().find_map(|run| {
                run.cursor_position(&cursor).map(|x| {
                    (
                        (rect.x() + x - offset).floor() as i32,
                        (rect.y() + run.line_top).floor() as i32,
                        1,
                        run.line_height.ceil().max(1.0) as i32,
                    )
                })
            })
        });
        let surrounding = &self.input.context.as_ref().unwrap().surrounding;
        let content = if self.input.single_line {
            ContentType::Text
        } else {
            ContentType::Multiline
        };
        if previous.is_some_and(|old| {
            old.target_id == target_id
                && old.focus_id == focus.id
                && old.monitor_idx == focus.monitor_idx
                && old.surrounding == *surrounding
                && old.cursor_rect == cursor_rect
                && old.content == content
        }) {
            return None;
        }
        Some(Snapshot {
            target_id,
            focus_id: focus.id,
            monitor_idx: focus.monitor_idx,
            surrounding: surrounding.clone(),
            cursor_rect,
            cause,
            content,
        })
    }
}
