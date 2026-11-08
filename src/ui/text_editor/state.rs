//! 【界面控件】【编辑状态】保存多行文字与有容量限制的增量修改历史

use std::collections::VecDeque;

use cosmic_text::{
    Action, Attrs, Buffer, Change, Cursor, Edit, Editor, FontSystem, Metrics, Selection, Shaping,
};
use unicode_segmentation::UnicodeSegmentation;

const HISTORY_LIMIT: usize = 128;
const HISTORY_BYTES: usize = 4 * 1024 * 1024;
pub(super) const TEXT_LIMIT: usize = 8 * 1024 * 1024;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum HistoryKind {
    Other,
    Insert,
    InputMethod,
    SelectionDelete,
}

pub(super) struct HistoryEntry {
    change: Change,
    before: (Cursor, Selection),
    after: (Cursor, Selection),
    bytes: usize,
    kind: HistoryKind,
}

pub struct TextEditor {
    pub(super) editor: Editor<'static>,
    pub(super) style: super::EditorStyle,
    undo: VecDeque<HistoryEntry>,
    redo: Vec<HistoryEntry>,
    history_bytes: usize,
    size: (f32, f32),
    pub(super) input: super::input_method::InputState,
}

impl TextEditor {
    /// 【文本编辑】【光标字节位置】无参数，返回当前行光标的 UTF-8 字节偏移，供密码遮蔽显示定位
    pub fn cursor_offset(&self) -> usize {
        self.editor.cursor().index
    }
    /// 【界面控件】【单行模式】禁止软换行，fonts 用于重新排版；原有文字和编辑历史保持不变
    pub fn single_line(&mut self, fonts: &mut FontSystem) {
        self.input.single_line = true;
        self.editor
            .with_buffer_mut(|buffer| buffer.set_wrap(cosmic_text::Wrap::None));
        self.editor.shape_as_needed(fonts, false);
    }

    /// 【界面控件】【编辑初始化】用 text 与 fonts 创建多行编辑器；初始内容不进入撤销历史
    pub fn new(text: &str, fonts: &mut FontSystem) -> Self {
        Self::new_with_style(text, fonts, super::EditorStyle::default())
    }

    /// 【界面控件】【样式初始化】用 text、fonts 和局部 style 创建编辑器，返回无修改历史的实例，不改变其他编辑器
    pub fn new_with_style(text: &str, fonts: &mut FontSystem, style: super::EditorStyle) -> Self {
        let mut buffer = Buffer::new(fonts, Metrics::new(style.size, style.line_height));
        buffer.set_text(text, &Attrs::new(), Shaping::Advanced, None);
        buffer.set_size(Some(640.0), Some(360.0));
        let mut editor = Editor::new(buffer);
        editor.set_auto_indent(false);
        editor.shape_as_needed(fonts, false);
        Self {
            editor,
            style,
            undo: VecDeque::new(),
            redo: Vec::new(),
            history_bytes: 0,
            size: (640.0, 360.0),
            input: super::input_method::InputState::default(),
        }
    }

    /// 【界面控件】【单行初始化】用 text 和 fonts 创建 Qt 单行语义编辑器，限制三万二千七百六十七个 UTF-16 单元，返回编辑器
    pub fn new_single_line(text: &str, fonts: &mut FontSystem) -> Self {
        Self::new_single_line_with_style(text, fonts, super::EditorStyle::default())
    }

    /// 【界面控件】【单行样式】用 text、fonts、style 创建局部样式单行编辑器，保留原版 UTF-16 长度上限，返回独立编辑器
    pub fn new_single_line_with_style(
        text: &str,
        fonts: &mut FontSystem,
        style: super::EditorStyle,
    ) -> Self {
        let mut value = Self::new_with_style(
            super::input_method::composition::clipped(text, 32767),
            fonts,
            style,
        );
        value.single_line(fonts);
        value
    }

    /// 【界面控件】【完整文字】返回当前全部文字，按原有行结束符连接
    pub fn text(&self) -> String {
        self.editor.with_buffer(|buffer| {
            let mut text = String::new();
            for line in &buffer.lines {
                text.push_str(line.text());
                text.push_str(line.ending().as_str());
            }
            text
        })
    }

    /// 【界面控件】【空文本判断】返回正文是否为空字符串，无参数，空白或换行仍属于文本，不分配全文副本
    pub fn is_empty(&self) -> bool {
        self.editor.with_buffer(|buffer| {
            buffer
                .lines
                .iter()
                .all(|line| line.text().is_empty() && line.ending().as_str().is_empty())
        })
    }

    /// 【界面控件】【正文资格】返回是否存在非空白字符；只借用行文本，无参数，不分配全文副本
    pub fn has_non_whitespace(&self) -> bool {
        self.editor.with_buffer(|buffer| {
            buffer
                .lines
                .iter()
                .any(|line| !line.text().trim().is_empty())
        })
    }

    /// 【界面控件】【选择文字】返回当前选中文字；未选择或空选择返回空值
    pub fn selected_text(&self) -> Option<String> {
        self.editor.copy_selection().filter(|text| !text.is_empty())
    }

    /// 【界面控件】【编辑尺寸】按 width、height 更新换行与视区；保留文字、选择及滚动位置
    pub fn resize(&mut self, fonts: &mut FontSystem, width: f32, height: f32) {
        let size = (width.max(1.0), height.max(1.0));
        if self.size != size {
            self.size = size;
            self.editor
                .with_buffer_mut(|buffer| buffer.set_size(Some(size.0), Some(size.1)));
            self.editor.shape_as_needed(fonts, false);
            self.resize_presentation(fonts, size.0, size.1);
        }
    }

    /// 【界面控件】【视区复位】把视区移到文字起点；不改变光标、选择或历史
    pub fn reset_scroll(&mut self) {
        self.editor
            .with_buffer_mut(|buffer| buffer.set_scroll(cosmic_text::Scroll::default()));
    }

    /// 【界面控件】【增量修改】执行 operation 并记录真实文字变化；fonts 用于排版，返回是否改变文字
    fn mutate(
        &mut self,
        fonts: &mut FontSystem,
        operation: impl FnOnce(&mut Editor<'static>, &mut FontSystem),
    ) -> bool {
        self.interrupt_input();
        let before = (!self.input.single_line).then(|| {
            (
                self.editor
                    .selection_bounds()
                    .map_or(self.editor.cursor(), |(_, end)| end),
                Selection::None,
            )
        });
        self.mutate_kind(fonts, HistoryKind::Other, before, operation)
    }

    /// 【界面控件】【历史分组】用 fonts 执行 operation，kind 和 before 控制 Qt 分组及撤销光标，返回是否改变正文
    pub(super) fn mutate_kind(
        &mut self,
        fonts: &mut FontSystem,
        kind: HistoryKind,
        before: Option<(Cursor, Selection)>,
        operation: impl FnOnce(&mut Editor<'static>, &mut FontSystem),
    ) -> bool {
        let before = before.unwrap_or_else(|| (self.editor.cursor(), self.editor.selection()));
        self.editor.start_change();
        operation(&mut self.editor, fonts);
        let change = self.editor.finish_change();
        self.editor.shape_as_needed(fonts, false);
        let Some(change) = change.filter(|change| !change.items.is_empty()) else {
            return false;
        };
        let bytes = change.items.iter().map(|item| item.text.len()).sum();
        let after = (self.editor.cursor(), self.editor.selection());
        let merge = !self.input.separator
            && self.undo.back().is_some_and(|previous| {
                previous.after == before
                    && before.1 == Selection::None
                    && ((previous.kind == kind && kind == HistoryKind::Insert)
                        || (self.input.single_line
                            && previous.kind == HistoryKind::SelectionDelete
                            && kind == HistoryKind::Insert))
            });
        if merge {
            let previous = self.undo.back_mut().unwrap();
            previous.change.items.extend(change.items);
            previous.after = after;
            previous.bytes += bytes;
            previous.kind = kind;
        } else {
            self.undo.push_back(HistoryEntry {
                change,
                before,
                after,
                bytes,
                kind,
            });
        }
        self.input.separator = kind == HistoryKind::Other;
        self.input.revision = self.input.revision.wrapping_add(1);
        self.redo.clear();
        self.history_bytes += bytes;
        while self.undo.len() > 1
            && (self.undo.len() > HISTORY_LIMIT || self.history_bytes > HISTORY_BYTES)
        {
            if let Some(entry) = self.undo.pop_front() {
                self.history_bytes -= entry.bytes;
            }
        }
        true
    }

    /// 【界面控件】【连续键入】用 fonts 插入 text 并按 Qt 连续文字分组，返回是否改变内容
    pub(super) fn type_text(&mut self, fonts: &mut FontSystem, text: &str) -> bool {
        let available = if self.input.single_line {
            32767usize.saturating_sub(
                self.text().encode_utf16().count().saturating_sub(
                    self.selected_text()
                        .map_or(0, |text| text.encode_utf16().count()),
                ),
            )
        } else {
            usize::MAX
        };
        let text = super::input_method::composition::clipped(text, available);
        if text.is_empty() {
            return false;
        }
        let length = self
            .text()
            .len()
            .saturating_sub(self.selected_text().map_or(0, |value| value.len()));
        if length.saturating_add(text.len()) > TEXT_LIMIT {
            return false;
        }
        let before = (!self.input.single_line).then(|| {
            (
                self.editor
                    .selection_bounds()
                    .map_or(self.editor.cursor(), |(_, end)| end),
                Selection::None,
            )
        });
        self.mutate_kind(fonts, HistoryKind::Insert, before, |editor, _| {
            editor.insert_string(text, None)
        })
    }

    /// 【界面控件】【历史光标】保存 position 为上一动作的重做位置，并分隔下一动作；用于单行选区删除后相对替换，无返回值
    pub(super) fn history_position(&mut self, position: (Cursor, Selection)) {
        if let Some(previous) = self.undo.back_mut() {
            previous.after = position;
        }
        self.input.separator = true;
    }

    /// 【界面控件】【动作执行】执行 action 并记录可撤销修改；返回文字是否发生变化
    pub fn action(&mut self, fonts: &mut FontSystem, action: Action) -> bool {
        if let Action::Insert(character) = action {
            return self.insert(fonts, &character.to_string());
        }
        if action == Action::Enter {
            return self.insert(fonts, "\n");
        }
        self.mutate(fonts, |editor, fonts| {
            if action == Action::Backspace {
                delete_previous_grapheme(editor);
            } else {
                editor.action(fonts, action);
            }
        })
    }

    /// 【界面控件】【文字插入】将 text 插入当前位置或替换选区；规范换行并拒绝超过容量上限的输入
    pub fn insert(&mut self, fonts: &mut FontSystem, text: &str) -> bool {
        let text = if self.input.single_line {
            text.to_owned()
        } else {
            text.replace("\r\n", "\n").replace('\r', "\n")
        };
        let length = self.editor.with_buffer(|buffer| {
            buffer
                .lines
                .iter()
                .map(|line| line.text().len() + line.ending().as_str().len())
                .sum::<usize>()
        });
        let selected = self.selected_text().map_or(0, |text| text.len());
        let text = if self.input.single_line {
            let available = 32767usize.saturating_sub(
                self.text().encode_utf16().count().saturating_sub(
                    self.selected_text()
                        .map_or(0, |value| value.encode_utf16().count()),
                ),
            );
            super::input_method::composition::clipped(&text, available)
        } else {
            &text
        };
        if text.is_empty()
            || length.saturating_sub(selected).saturating_add(text.len()) > TEXT_LIMIT
        {
            return false;
        }
        self.mutate(fonts, |editor, _| editor.insert_string(text, None))
    }

    /// 【界面控件】【选区删除】删除当前选择并记录撤销；返回是否删除文字
    pub fn delete_selection(&mut self, fonts: &mut FontSystem) -> bool {
        self.mutate(fonts, |editor, _| {
            editor.delete_selection();
        })
    }

    /// 【界面控件】【单词删除】按 backward 删除相邻单词或已有选区；返回文字是否变化
    pub fn delete_word(&mut self, fonts: &mut FontSystem, backward: bool) -> bool {
        self.mutate(fonts, |editor, fonts| {
            if editor.selection_bounds().is_none() {
                editor.set_selection(Selection::Normal(editor.cursor()));
                editor.action(
                    fonts,
                    Action::Motion(if backward {
                        cosmic_text::Motion::LeftWord
                    } else {
                        cosmic_text::Motion::RightWord
                    }),
                );
            }
            editor.delete_selection();
        })
    }

    /// 【界面控件】【全部选择】选择全部文字并将视区移动到末尾；不修改内容
    pub fn select_all(&mut self, fonts: &mut FontSystem) {
        self.interrupt_input();
        self.editor
            .set_selection(Selection::Normal(Cursor::new(0, 0)));
        self.editor
            .action(fonts, Action::Motion(cosmic_text::Motion::BufferEnd));
        self.editor.shape_as_needed(fonts, false);
    }

    /// 【界面控件】【历史可用】返回当前是否能够撤销
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// 【界面控件】【重做可用】返回当前是否存在重做记录，不修改候选、正文或历史，无参数
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// 【界面控件】【撤销】反向应用最近修改并恢复光标、选择；返回是否成功
    pub fn undo(&mut self, fonts: &mut FontSystem) -> bool {
        self.interrupt_input();
        let Some(mut entry) = self.undo.pop_back() else {
            return false;
        };
        self.history_bytes -= entry.bytes;
        entry.change.reverse();
        if !self.editor.apply_change(&entry.change) {
            log::error!("【界面控件】【编辑撤销】增量修改无法应用");
            return false;
        }
        self.editor.set_cursor(entry.before.0);
        self.editor.set_selection(entry.before.1);
        self.editor.shape_as_needed(fonts, false);
        self.redo.push(entry);
        self.input.revision = self.input.revision.wrapping_add(1);
        true
    }

    /// 【界面控件】【重做】重新应用最近撤销的修改并恢复光标、选择；返回是否成功
    pub fn redo(&mut self, fonts: &mut FontSystem) -> bool {
        self.interrupt_input();
        let Some(mut entry) = self.redo.pop() else {
            return false;
        };
        entry.change.reverse();
        if !self.editor.apply_change(&entry.change) {
            log::error!("【界面控件】【编辑重做】增量修改无法应用");
            return false;
        }
        self.editor.set_cursor(entry.after.0);
        self.editor.set_selection(entry.after.1);
        self.editor.shape_as_needed(fonts, false);
        self.history_bytes += entry.bytes;
        self.undo.push_back(entry);
        self.input.revision = self.input.revision.wrapping_add(1);
        true
    }
}

/// 【界面控件】【字素删除】删除 editor 光标前的完整字素或已有选区；跨行时合并前一行
fn delete_previous_grapheme(editor: &mut Editor<'static>) {
    if editor
        .selection_bounds()
        .is_some_and(|(start, end)| start != end)
    {
        editor.delete_selection();
        return;
    }
    let cursor = editor.cursor();
    let previous = editor.with_buffer(|buffer| {
        if cursor.index > 0 {
            buffer.lines[cursor.line].text()[..cursor.index]
                .grapheme_indices(true)
                .next_back()
                .map(|(index, _)| Cursor::new(cursor.line, index))
        } else if cursor.line > 0 {
            Some(Cursor::new(
                cursor.line - 1,
                buffer.lines[cursor.line - 1].text().len(),
            ))
        } else {
            None
        }
    });
    if let Some(previous) = previous {
        editor.delete_range(previous, cursor);
        editor.set_cursor(previous);
        editor.set_selection(Selection::None);
    }
}
