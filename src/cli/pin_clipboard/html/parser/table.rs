//! 【贴图】【HTML表格解析】在解析器中构建表格行、单元格与背景

use super::super::super::html_style;
use super::super::model::{Align, Block, BlockKind, Span, TableCell, TableRow, TableStyle};
use super::super::tags::{finalize_spans, tag_background};
use super::super::token::Tag;
use super::{FrameKind, Parser};
use crate::theme::Rgba;

/// 【贴图】【表格构建】记录正在构建的表格容器背景、源宽度、填充标记、边框、列宽与行/单元格
#[derive(Default)]
pub(super) struct TableBuilder {
    pub(super) background: Option<Rgba>,
    pub(super) width: Option<f32>,
    pub(super) fill: bool,
    pub(super) border: Option<Rgba>,
    pub(super) columns: Vec<f32>,
    pub(super) rows: Vec<TableRow>,
    pub(super) row: Option<TableRow>,
    pub(super) cell: Option<CellDraft>,
}

/// 【贴图】【单元格草稿】记录尚未写出的单元格片段、对齐、背景、表头标记与源宽度
pub(super) struct CellDraft {
    pub(super) header: bool,
    pub(super) align: Align,
    pub(super) background: Option<Rgba>,
    pub(super) width: Option<f32>,
    pub(super) spans: Vec<Span>,
}

/// 【贴图】【宽度规格】区分像素宽度、百分比宽度与缺失的源宽度声明
enum WidthSpec {
    Px(f32),
    Percent(f32),
    None,
}

/// 【贴图】【宽度声明】按 style 的 width 优先、HTML width 属性兜底解析宽度规格
fn width_spec(tag: &Tag) -> WidthSpec {
    if let Some(style) = tag.attr("style") {
        for (property, value) in html_style::declarations(style) {
            if !property.eq_ignore_ascii_case("width") {
                continue;
            }
            if let Some(percent) = html_style::percent(value) {
                return WidthSpec::Percent(percent);
            }
            if let Some(px) = html_style::length_px(value) {
                return WidthSpec::Px(px);
            }
        }
    }
    if let Some(value) = tag.attr("width") {
        if let Some(percent) = html_style::percent(value) {
            return WidthSpec::Percent(percent);
        }
        if let Ok(px) = value.trim().parse::<f32>()
            && px.is_finite()
            && px >= 0.0
        {
            return WidthSpec::Px(px.clamp(0.0, 100_000.0));
        }
        if let Some(px) = html_style::length_px(value) {
            return WidthSpec::Px(px);
        }
    }
    WidthSpec::None
}

/// 【贴图】【表格宽度】按 tag 返回像素宽度与是否用百分比填充可用宽度
fn table_width(tag: &Tag) -> (Option<f32>, bool) {
    match width_spec(tag) {
        WidthSpec::Px(px) => (Some(px), false),
        WidthSpec::Percent(_) => (None, true),
        WidthSpec::None => (None, false),
    }
}

/// 【贴图】【单元宽度】按 tag 与 table_width 解析单元宽度，百分比仅在已知表宽时换算
fn cell_width(tag: &Tag, table_width: Option<f32>) -> Option<f32> {
    match width_spec(tag) {
        WidthSpec::Px(px) => Some(px),
        WidthSpec::Percent(percent) => table_width.map(|width| width * percent / 100.0),
        WidthSpec::None => None,
    }
}

/// 【贴图】【表格边框】按 style 边框颜色优先、非零 border 属性兜底，返回黑色或解析到的颜色
fn table_border(tag: &Tag) -> Option<Rgba> {
    if let Some(style) = tag.attr("style")
        && let Some(color) = style_border_color(style)
    {
        return Some(color);
    }
    let enabled = tag.attr("border").is_some_and(|value| {
        let value = value.trim();
        !value.is_empty() && value != "0"
    });
    enabled.then_some(Rgba(0, 0, 0, 255))
}

/// 【贴图】【边框颜色】按 style 查找首个带颜色的 border 声明，windowtext 映射为黑色
fn style_border_color(style: &str) -> Option<Rgba> {
    for (property, value) in html_style::declarations(style) {
        if !property.to_ascii_lowercase().starts_with("border") {
            continue;
        }
        for token in value.split(|ch: char| ch.is_ascii_whitespace() || ch == ',') {
            if token.is_empty() {
                continue;
            }
            if token.eq_ignore_ascii_case("windowtext") {
                return Some(Rgba(0, 0, 0, 255));
            }
            if let Some(color) = html_style::color(token) {
                return Some(color);
            }
        }
    }
    None
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
        let (width, fill) = table_width(tag);
        let border = table_border(tag);
        self.table = Some(TableBuilder {
            background,
            width,
            fill,
            border,
            ..TableBuilder::default()
        });
        self.table_depth = 1;
        self.push_frame("table", FrameKind::Table);
        self.enter_align_scope(tag);
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
        self.enter_align_scope(tag);
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
        let align = self.effective_align(tag);
        let background = tag_background(tag);
        let table_width = self.table.as_ref().and_then(|table| table.width);
        let width = cell_width(tag, table_width);
        self.push_frame(&tag.name, FrameKind::Cell);
        self.enter_align_scope(tag);
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
                width,
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
            width: cell.width,
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
        let derived = self
            .table
            .as_ref()
            .filter(|table| table.columns.is_empty())
            .and_then(|_| {
                row.cells
                    .iter()
                    .map(|cell| cell.width)
                    .collect::<Option<Vec<f32>>>()
            });
        if let Some(columns) = derived
            && let Some(table) = self.table.as_mut()
        {
            table.columns = columns;
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
            kind: BlockKind::Table {
                rows: table.rows,
                style: TableStyle {
                    width: table.width,
                    fill: table.fill,
                    columns: table.columns,
                    border: table.border,
                },
            },
            align: Align::Left,
            background: table.background,
            spans: Vec::new(),
        });
    }
}
