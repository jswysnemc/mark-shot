//! 【贴图】【HTML表格解析】在解析器中构建表格行、单元格与背景

use super::super::model::{Align, Block, BlockKind, Span, TableCell, TableRow};
use super::super::tags::{block_align, finalize_spans, tag_background};
use super::super::token::Tag;
use super::{FrameKind, Parser};
use crate::theme::Rgba;

/// 【贴图】【表格构建】记录正在构建的表格容器背景、已完成行与当前行/单元格
#[derive(Default)]
pub(super) struct TableBuilder {
    pub(super) background: Option<Rgba>,
    pub(super) rows: Vec<TableRow>,
    pub(super) row: Option<TableRow>,
    pub(super) cell: Option<CellDraft>,
}

/// 【贴图】【单元格草稿】记录尚未写出的单元格片段、对齐、背景与表头标记
pub(super) struct CellDraft {
    pub(super) header: bool,
    pub(super) align: Align,
    pub(super) background: Option<Rgba>,
    pub(super) spans: Vec<Span>,
}

impl Parser {
    /// 【贴图】【单元格状态】无参数，返回当前是否已打开一个可写入的表格单元格
    pub(super) fn cell_open(&self) -> bool {
        self.table_depth == 1
            && self
                .table
                .as_ref()
                .is_some_and(|table| table.cell.is_some())
    }

    /// 【贴图】【表格开始】按 tag 写出上一块并创建表格构建器，记录容器背景与样式
    pub(super) fn start_table(&mut self, tag: &Tag) {
        self.flush();
        let background = tag_background(tag).or(self.block_background);
        self.table = Some(TableBuilder {
            background,
            ..TableBuilder::default()
        });
        self.table_depth = 1;
        self.push_frame("table", FrameKind::Table);
        self.apply_tag_style(tag);
    }

    /// 【贴图】【行开始】按 tag 结束上一行并开始新行，记录行背景与样式
    pub(super) fn start_row(&mut self, tag: &Tag) {
        self.finish_cell();
        self.finish_row();
        let background = tag_background(tag);
        if let Some(table) = self.table.as_mut() {
            table.row = Some(TableRow {
                header: false,
                background,
                cells: Vec::new(),
            });
        }
        self.push_frame("tr", FrameKind::Row);
        self.apply_tag_style(tag);
    }

    /// 【贴图】【单元开始】按 tag 结束上一单元并开始新单元，记录对齐、背景与表头默认字重
    pub(super) fn start_cell(&mut self, tag: &Tag) {
        self.finish_cell();
        if self.table.as_ref().is_some_and(|table| table.row.is_none()) {
            let background = self.table.as_ref().and_then(|table| table.background);
            if let Some(table) = self.table.as_mut() {
                table.row = Some(TableRow {
                    header: false,
                    background,
                    cells: Vec::new(),
                });
            }
        }
        let header = tag.name == "th";
        let align = block_align(tag);
        let background = tag_background(tag);
        self.push_frame(&tag.name, FrameKind::Cell);
        self.at_start = true;
        self.space = false;
        if header {
            self.style.bold = true;
        }
        self.apply_tag_style(tag);
        self.style.background = None;
        if let Some(table) = self.table.as_mut() {
            table.cell = Some(CellDraft {
                header,
                align,
                background,
                spans: Vec::new(),
            });
        }
    }

    /// 【贴图】【单元内块】按 tag 在单元格内插入换行而不创建顶层块，并沿用其内联样式
    pub(super) fn start_cell_block(&mut self, tag: &Tag) {
        let has_content = self
            .table
            .as_ref()
            .and_then(|table| table.cell.as_ref())
            .is_some_and(|cell| !cell.spans.is_empty());
        if has_content {
            self.newline();
        }
        if !tag.self_closing {
            self.push_frame(&tag.name, FrameKind::Inline);
        }
        self.apply_tag_style(tag);
    }

    /// 【贴图】【单元结束】无参数，裁剪尾部空白后把当前单元格写入所属行
    pub(super) fn finish_cell(&mut self) {
        let Some(mut cell) = self.table.as_mut().and_then(|table| table.cell.take()) else {
            return;
        };
        finalize_spans(&mut cell.spans, false);
        let header = cell.header;
        let Some(row) = self.table.as_mut().and_then(|table| table.row.as_mut()) else {
            return;
        };
        row.cells.push(TableCell {
            spans: cell.spans,
            align: cell.align,
            background: cell.background,
            header,
        });
        if header {
            row.header = true;
        }
    }

    /// 【贴图】【行结束】无参数，结束当前单元后把非空行写入表格
    pub(super) fn finish_row(&mut self) {
        self.finish_cell();
        let Some(row) = self.table.as_mut().and_then(|table| table.row.take()) else {
            return;
        };
        if row.cells.is_empty() {
            return;
        }
        if let Some(table) = self.table.as_mut() {
            table.rows.push(row);
        }
    }

    /// 【贴图】【表格结束】无参数，结束当前行后把含可见文本的表格写入块列表
    pub(super) fn finish_table(&mut self) {
        self.finish_row();
        let Some(table) = self.table.take() else {
            return;
        };
        let has_content = table.rows.iter().any(|row| {
            row.cells.iter().any(|cell| {
                cell.spans
                    .iter()
                    .any(|span| span.text.chars().any(|ch| !ch.is_whitespace()))
            })
        });
        if !has_content {
            return;
        }
        self.blocks.push(Block {
            kind: BlockKind::Table { rows: table.rows },
            align: Align::Left,
            background: table.background,
            spans: Vec::new(),
        });
    }
}
