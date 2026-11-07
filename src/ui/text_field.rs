use crate::types::SpecialKey;
use log::info;
/// another text engine, but unlike tool/text.rs, this is for simple single-line text input fields
/// so i couldn't use the same functions in both files, since tool one is for rich text and use editor
use std::collections::HashMap;
use std::hash::Hash;

pub mod composition;
mod history;
pub mod validation;
use history::{Delta, History, Kind, Position};

#[derive(Debug, Clone, Default)]
pub struct LineEditState {
    pub revision: u64,
    pub text: String,
    pub cursor: usize,
    pub selection_anchor: Option<usize>,
    pub composition: Option<composition::Composition>,
    history: History,
}

impl LineEditState {
    /// 【字段编辑】【字段构造】按 text 返回光标位于末尾的单行状态，不保留候选或撤销动作
    pub fn new(text: String) -> Self {
        let cursor = text.chars().count();
        Self {
            revision: 0,
            text,
            cursor,
            selection_anchor: None,
            composition: None,
            history: History::default(),
        }
    }

    /// 【字段编辑】【字符插入】把 ch 插入当前位置，替换选区并记录连续输入，无返回值
    pub fn insert(&mut self, ch: char) {
        self.insert_str(ch.encode_utf8(&mut [0; 4]));
    }

    /// 【字段编辑】【文本插入】把 s 插入当前位置并替换选区，候选先取消，连续输入合并撤销，无返回值
    pub fn insert_str(&mut self, s: &str) {
        self.revision = self.revision.wrapping_add(1);
        self.composition = None;
        self.delete_selection();
        let before = self.position();
        let byte_idx = self.byte_index(self.cursor);
        self.text.insert_str(byte_idx, s);
        self.cursor += s.chars().count();
        self.history.record(
            Delta {
                start: byte_idx,
                removed: String::new(),
                inserted: s.into(),
            },
            before,
            self.position(),
            Kind::Insert,
        );
    }

    /// 【字段编辑】【退格删除】删除选区或前一字符并记录历史，光标在开头时不修改，无参数、无返回值
    pub fn backspace(&mut self) {
        self.revision = self.revision.wrapping_add(1);
        self.composition = None;
        if self.delete_selection() {
            return;
        }
        if self.cursor == 0 {
            return;
        }
        let byte_idx = self.byte_index(self.cursor - 1);
        let next_byte_idx = self.byte_index(self.cursor);
        let before = self.position();
        let removed = self.text[byte_idx..next_byte_idx].to_owned();
        self.text.replace_range(byte_idx..next_byte_idx, "");
        self.cursor -= 1;
        self.history.record(
            Delta {
                start: byte_idx,
                removed,
                inserted: String::new(),
            },
            before,
            self.position(),
            Kind::Backspace,
        );
    }

    /// 【字段编辑】【向前删除】删除选区或后一字符并记录历史，光标在末尾时不修改，无参数、无返回值
    pub fn delete_forward(&mut self) {
        self.revision = self.revision.wrapping_add(1);
        self.composition = None;
        if self.delete_selection() {
            return;
        }
        let len = self.text.chars().count();
        if self.cursor >= len {
            return;
        }
        let byte_idx = self.byte_index(self.cursor);
        let next_byte_idx = self.byte_index(self.cursor + 1);
        let before = self.position();
        let removed = self.text[byte_idx..next_byte_idx].to_owned();
        self.text.replace_range(byte_idx..next_byte_idx, "");
        self.history.record(
            Delta {
                start: byte_idx,
                removed,
                inserted: String::new(),
            },
            before,
            self.position(),
            Kind::Delete,
        );
    }

    // ── Moving cursor stuff ─────────────────────────────────

    /// 【字段编辑】【左移光标】按 extend 扩展选区或收起到左边界，无返回值；存在选区时不额外移动一个字符
    pub fn move_left(&mut self, extend: bool) {
        let selected = (!extend).then(|| self.selection_range()).flatten();
        self.begin_or_clear_selection(extend);
        if let Some((start, _)) = selected {
            self.cursor = start;
        } else if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    /// 【字段编辑】【右移光标】按 extend 扩展选区或收起到右边界，无返回值；存在选区时不额外移动一个字符
    pub fn move_right(&mut self, extend: bool) {
        let selected = (!extend).then(|| self.selection_range()).flatten();
        self.begin_or_clear_selection(extend);
        let len = self.text.chars().count();
        if let Some((_, end)) = selected {
            self.cursor = end;
        } else if self.cursor < len {
            self.cursor += 1;
        }
    }

    pub fn move_home(&mut self, extend: bool) {
        self.begin_or_clear_selection(extend);
        self.cursor = 0;
    }

    pub fn move_end(&mut self, extend: bool) {
        self.begin_or_clear_selection(extend);
        self.cursor = self.text.chars().count();
    }

    /// 【字段编辑】【指定位置】按 index 与 extend 移动光标或扩展选择，限制到正文范围并分隔历史，无返回值
    pub fn move_to(&mut self, index: usize, extend: bool) {
        self.begin_or_clear_selection(extend);
        self.cursor = index.min(self.text.chars().count());
    }

    /// 【字段编辑】【全文选择】选择全部正文并分隔连续输入，取消候选，无参数、无返回值
    pub fn select_all(&mut self) {
        self.revision = self.revision.wrapping_add(1);
        self.composition = None;
        self.history.separate();
        self.selection_anchor = Some(0);
        self.cursor = self.text.chars().count();
    }

    /// 【字段编辑】【导航选择】按 extend 建立或清理选区，取消候选并分隔历史，无返回值
    fn begin_or_clear_selection(&mut self, extend: bool) {
        self.revision = self.revision.wrapping_add(1);
        self.composition = None;
        self.history.separate();
        if extend {
            if self.selection_anchor.is_none() {
                self.selection_anchor = Some(self.cursor);
            }
        } else {
            self.selection_anchor = None;
        }
    }

    // ── selection ─────────────────────────────────────────

    pub fn selection_range(&self) -> Option<(usize, usize)> {
        let anchor = self.selection_anchor?;
        if anchor == self.cursor {
            return None;
        }
        Some((anchor.min(self.cursor), anchor.max(self.cursor)))
    }

    pub fn selection_byte_range(&self) -> Option<(usize, usize)> {
        let (a, b) = self.selection_range()?;
        Some((self.byte_index(a), self.byte_index(b)))
    }

    pub fn selected_text(&self) -> Option<String> {
        let (a, b) = self.selection_byte_range()?;
        Some(self.text[a..b].to_string())
    }

    /// 【字段编辑】【选区删除】删除有效选区并记录原光标和锚点，返回是否修改正文，无参数
    fn delete_selection(&mut self) -> bool {
        let Some((start_char, _end_char)) = self.selection_range() else {
            return false;
        };
        let Some((start_byte, end_byte)) = self.selection_byte_range() else {
            return false;
        };
        let before = self.position();
        let removed = self.text[start_byte..end_byte].to_owned();
        self.text.replace_range(start_byte..end_byte, "");
        self.cursor = start_char;
        self.selection_anchor = None;
        self.history.record(
            Delta {
                start: start_byte,
                removed,
                inserted: String::new(),
            },
            before,
            self.position(),
            Kind::SelectionDelete,
        );
        true
    }

    pub fn cursor_byte(&self) -> usize {
        self.byte_index(self.cursor)
    }

    fn byte_index(&self, char_idx: usize) -> usize {
        self.text
            .char_indices()
            .nth(char_idx)
            .map(|(b, _)| b)
            .unwrap_or(self.text.len())
    }
    pub fn backspace_selection_only(&mut self) {
        self.revision = self.revision.wrapping_add(1);
        self.composition = None;
        self.delete_selection();
    }

    /// 【字段编辑】【位置读取】返回当前光标与有效选区锚点，无参数，不把零长度选区保存为历史选择
    fn position(&self) -> Position {
        Position {
            cursor: self.cursor,
            anchor: self
                .selection_anchor
                .filter(|anchor| *anchor != self.cursor),
        }
    }

    /// 【字段编辑】【撤销执行】恢复上一组正文、光标与选区并取消候选，返回是否存在有效历史，无参数
    pub fn undo(&mut self) -> bool {
        self.revision = self.revision.wrapping_add(1);
        self.composition = None;
        let Some(position) = self.history.undo(&mut self.text) else {
            return false;
        };
        self.cursor = position.cursor;
        self.selection_anchor = position.anchor;
        true
    }

    /// 【字段编辑】【重做执行】恢复下一组正文、光标与选区并取消候选，返回是否存在有效历史，无参数
    pub fn redo(&mut self) -> bool {
        self.revision = self.revision.wrapping_add(1);
        self.composition = None;
        let Some(position) = self.history.redo(&mut self.text) else {
            return false;
        };
        self.cursor = position.cursor;
        self.selection_anchor = position.anchor;
        true
    }

    /// 【字段编辑】【撤销能力】返回当前是否可撤销，无参数
    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    /// 【字段编辑】【重做能力】返回当前是否可重做，无参数
    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    /// 【字段编辑】【原生输入】按 event 与 rule 原子处理确认和候选，返回参数与正文是否有效；不逐字符过滤非法确认
    pub fn input_method(
        &mut self,
        event: crate::types::text_input::Edit,
        rule: validation::Rule,
    ) -> bool {
        composition::apply(self, event, rule).accepted
    }

    /// 【字段编辑】【输入更新】按 event 与 rule 返回正文、显示和验证变化，调用方据此更新过滤或数值；候选不视为正文修改
    pub fn input_method_update(
        &mut self,
        event: crate::types::text_input::Edit,
        rule: validation::Rule,
    ) -> composition::Update {
        composition::apply(self, event, rule)
    }

    /// 【字段编辑】【候选取消】只清除候选显示，返回是否存在候选；不恢复已删除选区，无参数
    pub fn cancel_preedit(&mut self) -> bool {
        let changed = self.composition.take().is_some();
        if changed {
            self.revision = self.revision.wrapping_add(1);
        }
        changed
    }

    /// 【字段编辑】【显示光标】返回候选或正文的 UTF-8 光标，显式隐藏时返回空值，无参数
    pub fn displayed_cursor(&self) -> Option<usize> {
        self.composition.as_ref().map_or_else(
            || Some(self.cursor_byte()),
            |composition| {
                composition
                    .preedit
                    .cursor
                    .map(|(begin, _)| composition.position + begin)
            },
        )
    }

    /// 【字段编辑】【显示选区】返回候选选择或实际正文选区的显示字节范围，无参数；插入候选不吞掉验证回退后的正文选区
    pub fn displayed_selection(&self) -> Option<(usize, usize)> {
        let Some(composition) = &self.composition else {
            return self.selection_byte_range();
        };
        if let Some((begin, end)) = composition.preedit.cursor
            && begin != end
        {
            return Some((
                composition.position + begin.min(end),
                composition.position + begin.max(end),
            ));
        }
        let (start, end) = self.selection_byte_range()?;
        Some((
            start
                + if start >= composition.position {
                    composition.preedit.text.len()
                } else {
                    0
                },
            end + if end > composition.position {
                composition.preedit.text.len()
            } else {
                0
            },
        ))
    }
}

pub fn is_stepper_char(ch: char) -> bool {
    ch.is_ascii_digit() || ch == '.' || ch == '-'
}

#[derive(Debug, Clone, Copy)]
pub enum CursorInit {
    End,
    SelectAll,
    At(usize),
}

pub struct FieldEdit<K> {
    pub id: u64,
    pub key: K,
    pub field: LineEditState,
}

pub struct TextFieldGroup<K: Eq + Hash + Copy> {
    pub values: HashMap<K, String>,
    pub editing: Option<FieldEdit<K>>,
}

impl<K: Eq + Hash + Copy> Default for TextFieldGroup<K> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Eq + Hash + Copy> TextFieldGroup<K> {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
            editing: None,
        }
    }

    /// 【字段编辑】【开始输入】按 key、initial_text 和 cursor 创建独立字段身份，无返回值；新字段不接收前一个字段的旧协议消息
    pub fn begin_edit(&mut self, key: K, initial_text: String, cursor: CursorInit) {
        let mut field = LineEditState::new(initial_text);
        match cursor {
            CursorInit::End => field.move_end(false),
            CursorInit::SelectAll => field.select_all(),
            CursorInit::At(idx) => {
                let len = field.text.chars().count();
                field.cursor = idx.min(len);
                field.selection_anchor = None;
            }
        }
        self.editing = Some(FieldEdit {
            key,
            field,
            id: crate::features::text::input_method::focus::identity(),
        });
    }

    pub fn commit_edit(&mut self) -> Option<(K, String)> {
        let edit = self.editing.take()?;
        Some((edit.key, edit.field.text))
    }

    pub fn is_editing(&self) -> bool {
        self.editing.is_some()
    }

    pub fn is_editing_key(&self, key: K) -> bool {
        self.editing.as_ref().map(|e| e.key) == Some(key)
    }

    pub fn insert_char(&mut self, ch: char, allowed: impl Fn(char) -> bool) -> bool {
        let Some(edit) = self.editing.as_mut() else {
            return false;
        };
        if !allowed(ch) {
            return false;
        }
        edit.field.insert(ch);
        true
    }

    pub fn handle_key(
        &mut self,
        key: SpecialKey,
        ctrl: bool,
        shift: bool,
        allowed: impl Fn(char) -> bool,
    ) -> (bool, bool) {
        let Some(edit) = self.editing.as_mut() else {
            return (false, false);
        };

        if ctrl {
            match key {
                SpecialKey::KeyA => {
                    edit.field.select_all();
                    return (true, false);
                }
                SpecialKey::KeyC | SpecialKey::KeyX => {
                    if let Some(sel) = edit.field.selected_text()
                        && let Err(e) = crate::utils::copy_to_clipboard(&sel)
                    {
                        info!("text field: can't copy: {e}");
                        return (false, false);
                    }
                    if matches!(key, SpecialKey::KeyX) {
                        edit.field.backspace_selection_only();
                        return (true, false);
                    }
                    return (false, false);
                }
                SpecialKey::KeyV => {
                    if let Some(text) = crate::utils::paste_from_clipboard() {
                        let filtered: String = text.chars().filter(|c| allowed(*c)).collect();
                        if !filtered.is_empty() {
                            edit.field.insert_str(&filtered);
                            return (true, false);
                        }
                    }
                    return (false, false);
                }
                _ => {}
            }
        }

        match key {
            SpecialKey::Enter => (false, true),
            SpecialKey::Left => {
                edit.field.move_left(shift);
                (true, false)
            }
            SpecialKey::Right => {
                edit.field.move_right(shift);
                (true, false)
            }
            SpecialKey::Home => {
                edit.field.move_home(shift);
                (true, false)
            }
            SpecialKey::End => {
                edit.field.move_end(shift);
                (true, false)
            }
            SpecialKey::Backspace => {
                edit.field.backspace();
                (true, false)
            }
            SpecialKey::Delete => {
                edit.field.delete_forward();
                (true, false)
            }
            _ => (false, false),
        }
    }

    pub fn sync_value(&mut self, key: K, text: String) {
        if self.is_editing_key(key) {
            return;
        }
        self.values.insert(key, text);
    }

    pub fn value(&self, key: K) -> Option<&String> {
        self.values.get(&key)
    }

    /// overwites editing buffer (text + cursor at end, selection cleared),
    /// even if the same key is being edited.
    /// User can type anything (including non-number), arrow key will still step
    /// from last confirmed value and replace buffer with result
    /// otherwise on commit old stuff text will overwrite what arrow key applied.
    pub fn set_editing_text(&mut self, key: K, text: String) {
        if let Some(edit) = self.editing.as_mut()
            && edit.key == key
        {
            edit.field = LineEditState::new(text);
            edit.id = crate::features::text::input_method::focus::identity();
        }
    }
}

pub fn is_hex_char(ch: char) -> bool {
    ch.is_ascii_hexdigit() || ch == '#'
}

pub fn is_rgba_channel_char(ch: char) -> bool {
    ch.is_ascii_digit()
}
