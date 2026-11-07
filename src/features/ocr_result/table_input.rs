//! 【文字识别】【单元格事务】表格选择与编辑分离，确认前不写持久单元格，取消不提交草稿

use super::table_view::TableView;
use crate::{
    backend::window::KeyInput,
    keys::Key,
    ui::text_editor::{EditEvent, EditorStyle, TextEditor},
};
use cosmic_text::FontSystem;
use tiny_skia::Rect;

impl TableView {
    /// 【文字识别】【编辑完成】按 commit 提交或丢弃当前单元格草稿，返回是否关闭编辑器；保留当前选择
    pub fn finish(&mut self, commit: bool) -> bool {
        if self.editor.is_none() {
            return false;
        }
        if commit {
            self.sync();
        }
        self.editor = None;
        self.editor_offset = 0.0;
        true
    }

    /// 【文字识别】【编辑开启】用 fonts 打开当前单元格的单行草稿，选择全部原文，返回是否成功
    pub(super) fn begin_edit(&mut self, fonts: &mut FontSystem) -> bool {
        let Some((row, column)) = self.active else {
            return false;
        };
        if self.editor.is_some() {
            return true;
        }
        let colors = super::palette::Palette::current();
        let mut editor = TextEditor::new_single_line_with_style(
            &self.table.rows[row][column],
            fonts,
            EditorStyle {
                size: super::table_grid_layout::CELL_FONT,
                line_height: 18.0,
                colors: Some([colors.text, colors.text, colors.accent, colors.on_accent]),
            },
        );
        editor.select_all(fonts);
        self.editor = Some(editor);
        self.editor_offset = 0.0;
        true
    }

    /// 【文字识别】【输入矩形】返回 body 中当前单元格的表面逻辑文字矩形，无活动单元格时返回空值
    pub(super) fn input_rect(&self, body: Rect) -> Option<Rect> {
        let (row, column) = self.active?;
        let local = self.text_rect(body, row, column);
        Rect::from_xywh(
            body.x() + local.x(),
            body.y() + local.y(),
            local.width(),
            local.height(),
        )
    }

    /// 【文字识别】【表格鼠标】在 body 内按 position 选择，双击才开启编辑；fonts、drag、clicks 控制字符选择，无返回值
    pub fn pointer(
        &mut self,
        fonts: &mut FontSystem,
        body: Rect,
        position: (f64, f64),
        drag: bool,
        clicks: u8,
    ) {
        let cell = if drag {
            self.active
        } else {
            self.cell_at(body, position)
        };
        let Some(cell) = cell else {
            if !drag {
                self.finish(true);
            }
            return;
        };
        if self.active != Some(cell) {
            self.finish(true);
            self.active = Some(cell);
        }
        if !drag && clicks >= 2 && self.editor.is_none() {
            self.begin_edit(fonts);
            return;
        }
        let rect = self.input_rect(body).unwrap();
        if let Some(editor) = &mut self.editor {
            editor.resize(fonts, rect.width(), rect.height());
            editor.pointer(
                fonts,
                (
                    position.0 - f64::from(rect.x()) + f64::from(self.editor_offset),
                    position.1 - f64::from(rect.y()),
                ),
                drag,
                clicks,
            );
        }
    }

    /// 【文字识别】【表格按键】处理 input；Enter 结束编辑且保持选择，Tab 循环选择，Escape 丢弃，fonts 和 body 用于排版，返回事件
    pub fn key(&mut self, fonts: &mut FontSystem, body: Rect, input: &KeyInput) -> EditEvent {
        let Some((row, column)) = self.active else {
            return EditEvent::None;
        };
        if !input.mods.ctrl && !input.mods.alt {
            if input.key == Some(Key::Escape) {
                return if self.finish(false) {
                    EditEvent::Redraw
                } else {
                    EditEvent::None
                };
            }
            if input.key == Some(Key::Enter) {
                return if self.finish(true) {
                    EditEvent::Redraw
                } else {
                    EditEvent::None
                };
            }
            if input.key == Some(Key::Tab) {
                self.finish(true);
                let columns = self.table.columns();
                let total = self.table.rows.len() * columns;
                let index = row * columns + column;
                let next = if input.mods.shift {
                    (index + total - 1) % total
                } else {
                    (index + 1) % total
                };
                self.active = Some((next / columns, next % columns));
                self.ensure_visible(body);
                return EditEvent::Redraw;
            }
            if input.key == Some(Key::F(2)) {
                self.begin_edit(fonts);
                return EditEvent::Redraw;
            }
        }
        if self.editor.is_none() {
            let next = match input.key {
                Some(Key::Left) => Some((row, column.saturating_sub(1))),
                Some(Key::Right) => Some((row, (column + 1).min(self.table.columns() - 1))),
                Some(Key::Up) => Some((row.saturating_sub(1), column)),
                Some(Key::Down) => Some(((row + 1).min(self.table.rows.len() - 1), column)),
                _ => None,
            };
            if let Some(next) = next {
                self.active = Some(next);
                self.ensure_visible(body);
                return EditEvent::Redraw;
            }
            let text = input.text.chars().any(|ch| !ch.is_control());
            if !text
                || input.mods.logo
                || (input.mods.ctrl && !input.mods.alt)
                || (input.mods.alt && !input.mods.ctrl)
            {
                return EditEvent::None;
            }
            self.begin_edit(fonts);
        }
        self.editor
            .as_mut()
            .map_or(EditEvent::None, |editor| editor.key(fonts, input))
    }
}
