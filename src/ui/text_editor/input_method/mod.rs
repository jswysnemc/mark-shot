//! 【窗口输入法】【显示隔离】候选仅保存在显示编辑器，已确认正文和历史始终使用原编辑器

pub(super) mod composition;
pub(super) mod context;

use super::TextEditor;
use crate::{features::text::input_method::focus::identity, types::text_input::Preedit};
use cosmic_text::{Attrs, Buffer, Cursor, Edit as _, Editor, FontSystem, Selection, Shaping};

pub(super) struct Composition {
    pub preedit: Preedit,
    pub editor: Editor<'static>,
}

pub(super) struct Flat {
    revision: u64,
    position: (Cursor, Selection),
    editor: Editor<'static>,
}

pub(super) struct InputState {
    pub id: u64,
    pub document_id: u64,
    pub single_line: bool,
    pub revision: u64,
    pub separator: bool,
    pub composition: Option<Composition>,
    pub context: Option<context::Cache>,
    flat: Option<Flat>,
}

impl Default for InputState {
    /// 【窗口输入法】【初始状态】返回独立身份和空候选、空缓存，不扫描字体或创建协议对象，无参数
    fn default() -> Self {
        let id = identity();
        Self {
            id,
            document_id: id,
            single_line: false,
            revision: 0,
            separator: true,
            composition: None,
            context: None,
            flat: None,
        }
    }
}

/// 【窗口输入法】【单行显示】按 text 返回只替换显示控制字符的文字，保留业务字符串中的换行和制表符
fn flat_text(text: &str) -> String {
    text.replace(['\r', '\n', '\t'], " ")
}

impl TextEditor {
    /// 【窗口输入法】【字段身份】返回当前编辑器的稳定身份，无参数；协议目标由窗口另行分配
    pub fn input_identity(&self) -> u64 {
        self.input.id
    }

    /// 【窗口输入法】【确认版本】返回当前确认正文的版本，无参数；候选、选择及滚动不改变版本，窗口可据此避免重复复制正文
    pub fn text_revision(&self) -> u64 {
        self.input.revision
    }

    /// 【窗口输入法】【正文身份】返回正文身份与确认版本，无参数；候选取消可更换协议身份，但不能伪装成正文变更
    pub fn text_version(&self) -> (u64, u64) {
        (self.input.document_id, self.input.revision)
    }

    /// 【窗口输入法】【候选读取】返回未确认候选，不把它包含在 text、复制或表格导出中，无参数
    pub fn preedit(&self) -> Option<&Preedit> {
        self.input.composition.as_ref().map(|value| &value.preedit)
    }

    /// 【窗口输入法】【候选取消】销毁显示候选，返回是否存在候选；不恢复输入法已经删除的选区，无参数
    pub fn cancel_preedit(&mut self) -> bool {
        self.input.composition.take().is_some()
    }

    /// 【窗口输入法】【普通输入】取消候选并隔离旧输入目标，返回是否取消；只用于普通键盘、鼠标或程序修改，无参数
    pub(super) fn interrupt_input(&mut self) -> bool {
        let changed = self.cancel_preedit();
        if changed {
            self.input.id = identity();
        }
        self.input.separator = true;
        changed
    }

    /// 【窗口输入法】【显示尺寸】用 fonts、width 和 height 更新已有显示编辑器，保留候选与光标，不复制正文，无返回值
    pub(super) fn resize_presentation(&mut self, fonts: &mut FontSystem, width: f32, height: f32) {
        if let Some(value) = &mut self.input.composition {
            value
                .editor
                .with_buffer_mut(|buffer| buffer.set_size(Some(width), Some(height)));
            value.editor.shape_as_needed(fonts, false);
        }
        if let Some(value) = &mut self.input.flat {
            value
                .editor
                .with_buffer_mut(|buffer| buffer.set_size(Some(width), Some(height)));
            value.editor.shape_as_needed(fonts, false);
        }
    }

    /// 【窗口输入法】【显示缓冲】返回正文缓冲的显示副本；单行模式把控制字符绘成空格，不改变持久文字，无参数
    fn display_buffer(&self) -> Buffer {
        let mut buffer = self.editor.with_buffer(Clone::clone);
        if self.input.single_line {
            let text = flat_text(&self.text());
            let defaults = buffer
                .lines
                .first()
                .map(|line| cosmic_text::AttrsOwned::new(&line.attrs_list().defaults()));
            buffer.set_text(
                &text,
                &defaults
                    .as_ref()
                    .map_or(Attrs::new(), |value| value.as_attrs()),
                Shaping::Advanced,
                None,
            );
            buffer.set_wrap(cosmic_text::Wrap::None);
        }
        buffer
    }

    /// 【窗口输入法】【显示位置】把 committed 的正文光标转为当前显示位置，单行使用全局字节；非法位置返回空值
    fn display_cursor(&self, committed: Cursor) -> Option<Cursor> {
        if self.input.single_line {
            self.byte_offset(committed)
                .map(|index| Cursor::new(0, index))
        } else {
            Some(committed)
        }
    }

    /// 【窗口输入法】【绘制编辑器】按 fonts 返回候选或单行显示编辑器；只在内容、光标实际改变时重建单行副本
    pub(super) fn display_editor(&mut self, fonts: &mut FontSystem) -> &mut Editor<'static> {
        let flatten = self.input.composition.is_none()
            && self.input.single_line
            && self.editor.with_buffer(|buffer| {
                buffer.lines.len() > 1
                    || buffer
                        .lines
                        .iter()
                        .any(|line| line.text().contains(['\r', '\n', '\t']))
            });
        let position = (self.editor.cursor(), self.editor.selection());
        if flatten
            && self.input.flat.as_ref().is_none_or(|flat| {
                flat.revision != self.input.revision || flat.position != position
            })
        {
            let mut editor = Editor::new(self.display_buffer());
            let cursor = self.display_cursor(position.0).unwrap();
            editor.set_cursor(cursor);
            editor.set_selection(match position.1 {
                Selection::None => Selection::None,
                Selection::Normal(anchor) | Selection::Word(anchor) | Selection::Line(anchor) => {
                    Selection::Normal(self.display_cursor(anchor).unwrap())
                }
            });
            editor.shape_as_needed(fonts, false);
            self.input.flat = Some(Flat {
                revision: self.input.revision,
                position,
                editor,
            });
        }
        if let Some(value) = &mut self.input.composition {
            &mut value.editor
        } else if flatten {
            &mut self.input.flat.as_mut().unwrap().editor
        } else {
            &mut self.editor
        }
    }
}
