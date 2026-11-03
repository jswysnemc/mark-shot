//! 【窗口输入法】【原子编辑】先验证完整字节替换，再按 QTextEdit 或 QLineEdit 的实际分组确认

use super::super::{
    TextEditor,
    state::{HistoryKind, TEXT_LIMIT},
};
use super::{Composition, flat_text};
use crate::types::text_input::{Edit, Preedit, Replacement};
use cosmic_text::{Cursor, Edit as _, Editor, FontSystem, Selection, UnderlineStyle};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Update {
    pub accepted: bool,
    pub text_changed: bool,
    pub changed: bool,
}

struct Plan {
    editing: bool,
    selection: bool,
    start: Cursor,
    end: Cursor,
    inserted: String,
    tracking: Cursor,
    undo: (Cursor, Selection),
    replacement: bool,
}

/// 【窗口输入法】【长度裁切】按 maximum 个 UTF-16 单元裁切 text，返回合法 UTF-8 子串，不切开代理对
pub(in crate::ui::text_editor) fn clipped(text: &str, maximum: usize) -> &str {
    let mut units = 0;
    for (index, ch) in text.char_indices() {
        units += ch.len_utf16();
        if units > maximum {
            return &text[..index];
        }
    }
    text
}

/// 【窗口输入法】【候选位置】从 start 沿 prefix 前进，返回显示光标；prefix 已规范为有效文本
fn advance(start: Cursor, prefix: &str) -> Cursor {
    let mut parts = prefix.split('\n');
    let first = parts.next().unwrap_or_default();
    let mut cursor = Cursor::new(start.line, start.index + first.len());
    for part in parts {
        cursor = Cursor::new(cursor.line + 1, part.len());
    }
    cursor
}

/// 【窗口输入法】【完整计划】按 field 和 event 计算已选删除、相对替换与最终光标，返回计划；非法 UTF-8 或容量范围不修改状态
fn prepare(field: &TextEditor, event: &Edit) -> Option<Plan> {
    if let Some((start, end)) = event.preedit.cursor
        && (!event.preedit.text.is_char_boundary(start)
            || !event.preedit.text.is_char_boundary(end))
    {
        return None;
    }
    if event.preedit.text.len() > TEXT_LIMIT {
        return None;
    }
    let original = field.editor.cursor();
    let selection = field
        .editor
        .selection_bounds()
        .filter(|(start, end)| start != end);
    let previous = field.preedit().map_or("", |value| &value.text);
    let replacement = event.replacement.unwrap_or(Replacement {
        start: 0,
        length: 0,
    });
    let editing =
        !event.commit.is_empty() || replacement.length != 0 || event.preedit.text != previous;
    let selected = editing && selection.is_some();
    let undo = if field.input.single_line {
        (original, field.editor.selection())
    } else {
        (selection.map_or(original, |(_, end)| end), Selection::None)
    };
    if !selected && event.commit.is_empty() && replacement.start == 0 && replacement.length == 0 {
        return Some(Plan {
            editing,
            selection: false,
            start: original,
            end: original,
            inserted: String::new(),
            tracking: original,
            undo,
            replacement: false,
        });
    }
    let mut text = field.text();
    let mut cursor = field.byte_offset(original)?;
    if selected {
        let (start, end) = selection.unwrap();
        let start = field.byte_offset(start)?;
        let end = field.byte_offset(end)?;
        text.replace_range(start..end, "");
        cursor = start;
    }
    let replacing =
        editing && (!event.commit.is_empty() || replacement.start != 0 || replacement.length != 0);
    let start = if replacing {
        cursor.checked_add_signed(replacement.start)?
    } else {
        cursor
    };
    let end = if replacing {
        start.checked_add(replacement.length)?
    } else {
        start
    };
    let removed = text.get(start..end)?;
    let inserted = if field.input.single_line {
        let available = 32767usize.saturating_sub(
            text.encode_utf16()
                .count()
                .saturating_sub(removed.encode_utf16().count()),
        );
        clipped(&event.commit, available).to_owned()
    } else {
        event.commit.replace("\r\n", "\n").replace('\r', "\n")
    };
    if text
        .len()
        .saturating_sub(removed.len())
        .saturating_add(inserted.len())
        > TEXT_LIMIT
    {
        return None;
    }
    let start_cursor = super::context::cursor_in_text(&text, start)?;
    let end_cursor = super::context::cursor_in_text(&text, end)?;
    text.replace_range(start..end, &inserted);
    let tracking = if field.input.single_line || (cursor >= start && cursor <= end) {
        start + inserted.len()
    } else if cursor > end {
        cursor - (end - start) + inserted.len()
    } else {
        cursor
    };
    Some(Plan {
        editing,
        selection: selected,
        start: start_cursor,
        end: end_cursor,
        inserted,
        tracking: super::context::cursor_in_text(&text, tracking)?,
        undo,
        replacement: replacing,
    })
}

impl TextEditor {
    /// 【窗口输入法】【候选显示】用 fonts 显示 preedit，显示副本不进入撤销、复制或原文，无返回值
    fn show_preedit(&mut self, fonts: &mut FontSystem, preedit: Preedit) {
        let mut editor = Editor::new(self.display_buffer());
        let start = self.display_cursor(self.editor.cursor()).unwrap();
        editor.set_cursor(start);
        let displayed = if self.input.single_line {
            flat_text(&preedit.text)
        } else {
            preedit.text.replace("\r\n", "\n").replace('\r', "\n")
        };
        editor.insert_string(&displayed, None);
        let end = editor.cursor();
        editor.with_buffer_mut(|buffer| {
            for index in start.line..=end.line {
                let line = &mut buffer.lines[index];
                let from = if index == start.line { start.index } else { 0 };
                let to = if index == end.line {
                    end.index
                } else {
                    line.text().len()
                };
                if from < to {
                    let mut attrs = line.attrs_list().clone();
                    let defaults = cosmic_text::AttrsOwned::new(&attrs.defaults());
                    attrs.add_span(
                        from..to,
                        &defaults.as_attrs().underline(UnderlineStyle::Single),
                    );
                    line.set_attrs_list(attrs);
                }
            }
        });
        let normalize = |value: &str| {
            if self.input.single_line {
                flat_text(value)
            } else {
                value.replace("\r\n", "\n").replace('\r', "\n")
            }
        };
        let (begin, anchor) = preedit.cursor.map_or((start, start), |(begin, end)| {
            (
                advance(start, &normalize(&preedit.text[..begin])),
                advance(start, &normalize(&preedit.text[..end])),
            )
        });
        editor.set_cursor(begin);
        editor.set_selection(if begin == anchor {
            Selection::None
        } else {
            Selection::Normal(anchor)
        });
        editor.shape_as_needed(fonts, false);
        self.input.composition = Some(Composition { preedit, editor });
    }

    /// 【窗口输入法】【事件确认】用 fonts 应用 event，返回接受、正文变化与显示变化；非法批次整体拒绝，QTextEdit 确认独立撤销
    pub fn input_method(&mut self, fonts: &mut FontSystem, event: Edit) -> Update {
        let Some(plan) = prepare(self, &event) else {
            return Update::default();
        };
        let preedit_changed = self
            .preedit()
            .map_or(!event.preedit.text.is_empty(), |previous| {
                previous != &event.preedit
            });
        if !plan.editing && !preedit_changed {
            return Update {
                accepted: true,
                ..Update::default()
            };
        }
        let cancelled = self.cancel_preedit();
        let mut text_changed = false;
        if self.input.single_line {
            if plan.selection {
                text_changed |=
                    self.mutate_kind(fonts, HistoryKind::SelectionDelete, None, |editor, _| {
                        editor.delete_selection();
                    });
            }
            if plan.replacement {
                if plan.selection && (plan.start != self.editor.cursor() || plan.start != plan.end)
                {
                    self.history_position((
                        plan.start,
                        if plan.start == plan.end {
                            Selection::None
                        } else {
                            Selection::Normal(plan.end)
                        },
                    ));
                }
                self.editor.set_cursor(plan.start);
                self.editor.set_selection(if plan.start == plan.end {
                    Selection::None
                } else {
                    Selection::Normal(plan.end)
                });
                let kind = if plan.inserted.is_empty() {
                    HistoryKind::SelectionDelete
                } else {
                    HistoryKind::Insert
                };
                text_changed |= self.mutate_kind(fonts, kind, None, |editor, _| {
                    editor.delete_selection();
                    editor.insert_string(&plan.inserted, None);
                });
            }
        } else if plan.selection || plan.replacement {
            text_changed = self.mutate_kind(
                fonts,
                HistoryKind::InputMethod,
                Some(plan.undo),
                |editor, _| {
                    if plan.selection {
                        editor.delete_selection();
                    }
                    if plan.replacement {
                        editor.set_cursor(plan.start);
                        editor.set_selection(if plan.start == plan.end {
                            Selection::None
                        } else {
                            Selection::Normal(plan.end)
                        });
                        editor.delete_selection();
                        editor.insert_string(&plan.inserted, None);
                    }
                },
            );
        }
        self.editor.set_cursor(plan.tracking);
        if plan.editing {
            self.editor.set_selection(Selection::None);
        }
        self.editor.shape_as_needed(fonts, false);
        if !event.preedit.text.is_empty() {
            self.show_preedit(fonts, event.preedit);
        }
        Update {
            accepted: true,
            text_changed,
            changed: text_changed || preedit_changed || cancelled,
        }
    }
}
