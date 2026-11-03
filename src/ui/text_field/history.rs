//! 【字段编辑】【增量历史】保存实际删除与插入片段，连续确认合并撤销，撤销恢复原字符选区

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Position {
    pub cursor: usize,
    pub anchor: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Insert,
    SelectionDelete,
    Backspace,
    Delete,
}

#[derive(Clone, Debug)]
pub(super) struct Delta {
    pub start: usize,
    pub removed: String,
    pub inserted: String,
}

impl Delta {
    /// 【字段编辑】【片段恢复】按 undo 将当前片段恢复到 text，返回字节范围是否合法；非法状态不部分替换
    fn apply(&self, text: &mut String, undo: bool) -> bool {
        let (old, new) = if undo {
            (&self.inserted, &self.removed)
        } else {
            (&self.removed, &self.inserted)
        };
        let Some(end) = self.start.checked_add(old.len()) else {
            return false;
        };
        if text.get(self.start..end) != Some(old.as_str()) {
            return false;
        }
        text.replace_range(self.start..end, new);
        true
    }
}

#[derive(Clone, Debug)]
struct Group {
    deltas: Vec<Delta>,
    before: Position,
    after: Position,
    kind: Kind,
}

#[derive(Clone, Debug, Default)]
pub(super) struct History {
    groups: Vec<Group>,
    cursor: usize,
    separator: bool,
}

impl History {
    /// 【字段编辑】【动作记录】保存 delta 与前后位置，kind 控制连续输入分组，无返回值；选择删除与紧随确认可组成一个动作
    pub fn record(&mut self, delta: Delta, before: Position, after: Position, kind: Kind) {
        if delta.removed.is_empty() && delta.inserted.is_empty() {
            return;
        }
        self.groups.truncate(self.cursor);
        let merge = !self.separator
            && self.groups.last().is_some_and(|previous| {
                previous.after == before
                    && before.anchor.is_none()
                    && ((previous.kind == kind && kind != Kind::SelectionDelete)
                        || (previous.kind == Kind::SelectionDelete && kind == Kind::Insert))
            });
        if merge {
            let previous = self.groups.last_mut().unwrap();
            previous.deltas.push(delta);
            previous.after = after;
            previous.kind = kind;
        } else {
            self.groups.push(Group {
                deltas: vec![delta],
                before,
                after,
                kind,
            });
        }
        self.cursor = self.groups.len();
        self.separator = false;
    }

    /// 【字段编辑】【输入分隔】结束当前连续输入组，不增加历史动作，无参数、无返回值
    pub fn separate(&mut self) {
        self.separator = true;
    }

    /// 【字段编辑】【撤销读取】返回是否存在可撤销动作，无参数
    pub fn can_undo(&self) -> bool {
        self.cursor > 0
    }

    /// 【字段编辑】【重做读取】返回是否存在已撤销动作，无参数
    pub fn can_redo(&self) -> bool {
        self.cursor < self.groups.len()
    }

    /// 【字段编辑】【撤销执行】原子恢复 text 的上一动作，返回该动作之前的光标和选区；不合法或没有历史时返回空值
    pub fn undo(&mut self, text: &mut String) -> Option<Position> {
        let group = self.groups.get(self.cursor.checked_sub(1)?)?;
        let mut restored = text.clone();
        for delta in group.deltas.iter().rev() {
            if !delta.apply(&mut restored, true) {
                return None;
            }
        }
        *text = restored;
        self.cursor -= 1;
        self.separator = true;
        Some(group.before)
    }

    /// 【字段编辑】【重做执行】原子重放 text 的下一动作，返回该动作之后的光标和选区；不合法或没有历史时返回空值
    pub fn redo(&mut self, text: &mut String) -> Option<Position> {
        let group = self.groups.get(self.cursor)?;
        let mut restored = text.clone();
        for delta in &group.deltas {
            if !delta.apply(&mut restored, false) {
                return None;
            }
        }
        *text = restored;
        self.cursor += 1;
        self.separator = true;
        Some(group.after)
    }
}
