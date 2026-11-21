//! 【RTF解析】【表格构建】维护正文表格行和单元格

use super::super::html::{
    Align, Block, BlockKind, Span, SpanStyle, TableCell, TableRow, TableStyle,
};
use super::super::spans::{append_span, trim_spans};
use crate::theme::Rgba;

/// 【RTF解析】【单元格草稿】记录尚未结束的单元格片段、对齐、背景与表头标记
struct CellDraft {
    spans: Vec<Span>,
    align: Align,
    background: Option<Rgba>,
    header: bool,
}

/// 【RTF解析】【表格构建】维护已完成行、当前行与当前单元格，并处理 `\clcbpatN` 顺序
#[derive(Default)]
pub(super) struct TableBuilder {
    rows: Vec<TableRow>,
    row: Option<TableRow>,
    cell: Option<CellDraft>,
    backgrounds: Vec<Option<Rgba>>,
    index: usize,
    header: bool,
    intbl: bool,
}

impl TableBuilder {
    /// 【RTF解析】【新建表格】无参数，返回空表格构建器
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// 【RTF解析】【接收文本】无参数，返回当前段落是否应写入表格单元格
    pub(super) fn accepts_text(&self) -> bool {
        self.intbl || self.cell.is_some()
    }

    /// 【RTF解析】【活动行】无参数，返回是否仍有未结束的行、单元格或表格正文
    pub(super) fn active(&self) -> bool {
        self.row.is_some() || self.cell.is_some() || self.intbl
    }

    /// 【RTF解析】【表格段落】无参数，标记后续文本属于表格正文
    pub(super) fn mark_intbl(&mut self) {
        self.intbl = true;
        self.ensure_row();
    }

    /// 【RTF解析】【开始行】无参数，结束上一行并创建新的表格行
    pub(super) fn start_row(&mut self) {
        if self.row.is_some() {
            self.finish_cell();
            self.finish_row();
        }
        self.row = Some(TableRow {
            header: false,
            background: None,
            cells: Vec::new(),
        });
        self.backgrounds.clear();
        self.index = 0;
        self.header = false;
    }

    /// 【RTF解析】【表头行】无参数，把当前行标记为表头
    pub(super) fn mark_header(&mut self) {
        self.header = true;
        if let Some(row) = self.row.as_mut() {
            row.header = true;
        }
    }

    /// 【RTF解析】【单元背景】按 color 记录下一个单元格背景，按行定义顺序排列
    pub(super) fn cell_background(&mut self, color: Option<Rgba>) {
        self.backgrounds.push(color);
    }

    /// 【RTF解析】【追加片段】按 span 与 align 写入当前单元格，必要时延迟创建行与单元格
    pub(super) fn append_span(&mut self, span: Span, align: Align) {
        self.ensure_cell(align);
        if let Some(cell) = self.cell.as_mut() {
            append_span(&mut cell.spans, &span.text, &span.style);
        }
    }

    /// 【RTF解析】【追加换行】按 style 与 align 在单元格内追加换行
    pub(super) fn newline(&mut self, style: &SpanStyle, align: Align) {
        self.ensure_cell(align);
        if let Some(cell) = self.cell.as_mut() {
            append_span(&mut cell.spans, "\n", style);
        }
    }

    /// 【RTF解析】【结束单元格】无参数，裁剪尾部空白后把当前单元格写入所属行
    pub(super) fn finish_cell(&mut self) {
        let Some(row) = self.row.as_mut() else {
            return;
        };
        let header = row.header;
        let background = self.backgrounds.get(self.index).copied().flatten();
        let mut cell = self.cell.take().unwrap_or(CellDraft {
            spans: Vec::new(),
            align: Align::Left,
            background,
            header,
        });
        trim_spans(&mut cell.spans);
        row.cells.push(TableCell {
            spans: cell.spans,
            align: cell.align,
            background: cell.background,
            header: cell.header,
            width: None,
        });
        self.index = self.index.saturating_add(1);
    }

    /// 【RTF解析】【结束行】无参数，结束当前单元格与行并把行写入表格
    pub(super) fn finish_row(&mut self) {
        if self.row.is_none() {
            return;
        }
        if self.cell.is_some() {
            self.finish_cell();
        }
        if let Some(row) = self.row.take() {
            self.rows.push(row);
        }
        self.intbl = false;
    }

    /// 【RTF解析】【写出表格】无参数，结束残余行并返回表格块；没有行时返回空值
    pub(super) fn take_block(&mut self) -> Option<Block> {
        if self.row.is_some() || self.cell.is_some() {
            self.finish_row();
        }
        self.cell = None;
        self.intbl = false;
        self.backgrounds.clear();
        self.index = 0;
        self.header = false;
        (!self.rows.is_empty()).then(|| Block {
            kind: BlockKind::Table {
                rows: std::mem::take(&mut self.rows),
                style: TableStyle::default(),
            },
            align: Align::Left,
            background: None,
            spans: Vec::new(),
        })
    }

    /// 【RTF解析】【保证行】无参数，若尚未创建行则补建一行
    fn ensure_row(&mut self) {
        if self.row.is_none() {
            self.row = Some(TableRow {
                header: false,
                background: None,
                cells: Vec::new(),
            });
        }
    }

    /// 【RTF解析】【保证单元格】按 align 创建当前单元格，背景按行定义顺序取用
    fn ensure_cell(&mut self, align: Align) {
        self.ensure_row();
        if self.cell.is_some() {
            return;
        }
        let header = self.row.as_ref().is_some_and(|row| row.header);
        let background = self.backgrounds.get(self.index).copied().flatten();
        self.cell = Some(CellDraft {
            spans: Vec::new(),
            align,
            background,
            header,
        });
    }
}
