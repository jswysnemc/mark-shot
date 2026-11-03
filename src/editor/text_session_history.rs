//! 【文字事务】【局部历史】增量保存编辑器操作，连续输入合并，标注历史只在提交时改变

use super::text_session_format::Font;
use cosmic_text::{Change, Cursor, Edit, Editor, Selection};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Typing,
    Backspace,
    Delete,
    Standalone,
}

enum Operation {
    Text { change: Change, font: Font },
    Format { before: Font, after: Font },
}

struct Entry {
    operation: Operation,
    before: (Cursor, Selection),
    after: (Cursor, Selection),
    kind: Option<Kind>,
}

pub struct History {
    undo: Vec<Entry>,
    redo: Vec<Entry>,
    group: Option<Kind>,
}

impl History {
    /// 【文字事务】【初始格式】按 cursor、font 与 existing 返回原版初始化格式记录；已有文档两次设置格式，正文不进入修改历史
    pub fn new(cursor: Cursor, font: Font, existing: bool) -> Self {
        Self {
            undo: (0..if existing { 2 } else { 1 })
                .map(|_| Entry {
                    operation: Operation::Format {
                        before: font.clone(),
                        after: font.clone(),
                    },
                    before: (cursor, Selection::None),
                    after: (cursor, Selection::None),
                    kind: None,
                })
                .collect(),
            redo: Vec::new(),
            group: None,
        }
    }

    /// 【文字事务】【输入分组】结束当前连续输入组，无参数、无返回值，不清除可撤销记录
    pub fn break_group(&mut self) {
        self.group = None;
    }

    /// 【文字事务】【历史可用】返回当前能否执行撤销或重做，redo 为真时检查重做记录
    pub fn available(&self, redo: bool) -> bool {
        if redo {
            !self.redo.is_empty()
        } else {
            !self.undo.is_empty()
        }
    }

    /// 【文字事务】【增量记录】保存 change、font、before、after 与 kind；连续位置相同且字体一致的输入合并，空修改不清除重做，无返回值
    pub fn record(
        &mut self,
        change: Option<Change>,
        font: Font,
        before: (Cursor, Selection),
        after: (Cursor, Selection),
        kind: Kind,
    ) {
        let Some(change) = change.filter(|change| !change.items.is_empty()) else {
            return;
        };
        let merge = kind != Kind::Standalone
            && self.group == Some(kind)
            && self.undo.last().is_some_and(|entry| {
                entry.kind == Some(kind) && entry.after == before && before.1 == Selection::None
                    && matches!(&entry.operation,Operation::Text {font:previous,..} if *previous==font)
            });
        if merge {
            let entry = self.undo.last_mut().unwrap();
            let Operation::Text {
                change: previous, ..
            } = &mut entry.operation
            else {
                unreachable!()
            };
            // 1. 【文字事务】【连续输入】相邻插入合并为单个增量，避免为每个字符保存独立对象
            for item in change.items {
                if let Some(last) = previous.items.last_mut()
                    && last.insert
                    && item.insert
                    && last.end.line == item.start.line
                    && last.end.index == item.start.index
                    && item.start.line == item.end.line
                {
                    last.text.push_str(&item.text);
                    last.end = item.end;
                } else {
                    previous.items.push(item);
                }
            }
            entry.after = after;
        } else {
            self.undo.push(Entry {
                operation: Operation::Text { change, font },
                before,
                after,
                kind: Some(kind),
            });
        }
        self.group = (kind != Kind::Standalone).then_some(kind);
        self.redo.clear();
    }

    /// 【文字事务】【格式记录】保存 before、after 字体与 cursor，格式确认即使同值也独立可撤销，不复制正文，无返回值
    pub fn record_font(&mut self, before: Font, after: Font, cursor: Cursor) {
        self.undo.push(Entry {
            operation: Operation::Format { before, after },
            before: (cursor, Selection::None),
            after: (cursor, Selection::None),
            kind: None,
        });
        self.group = None;
        self.redo.clear();
    }

    /// 【文字事务】【局部恢复】在 editor 上执行 redo 指定的历史方向，恢复光标和选区；返回是否存在可用记录
    pub fn step(
        &mut self,
        editor: &mut Editor<'static>,
        working: &mut crate::types::Annotation,
        redo: bool,
    ) -> bool {
        self.break_group();
        let entry = if redo {
            self.redo.pop()
        } else {
            self.undo.pop()
        };
        let Some(mut entry) = entry else {
            return false;
        };
        let format = matches!(entry.operation, Operation::Format { .. });
        match &mut entry.operation {
            Operation::Format { before, after } => if redo { after } else { before }.apply(working),
            Operation::Text { change, font } => {
                change.reverse();
                if !editor.apply_change(change) {
                    log::error!("【文字事务】【局部恢复】增量修改无法应用");
                    return false;
                }
                font.apply(working);
            }
        }
        let position = if redo { entry.after } else { entry.before };
        editor.set_cursor(position.0);
        editor.set_selection(position.1);
        if redo {
            self.undo.push(entry);
        } else {
            self.redo.push(entry);
            if format {
                // 1. 【文字事务】【恢复输入组】格式撤销后回到此前文档，续写可与相同位置的原输入组合并
                self.group = self.undo.last().and_then(|entry| entry.kind);
            }
        }
        true
    }
}
