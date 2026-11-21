//! 【贴图】【HTML模型】定义解析后的富文本、块、表格与片段样式结构

use crate::theme::Rgba;

/// 【贴图】【富文本】记录解析后的段落列表及外层容器画布与默认文字颜色
#[derive(Clone, Debug, Default, PartialEq)]
pub(in crate::cli::pin_clipboard) struct RichText {
    pub blocks: Vec<Block>,
    pub background: Option<Rgba>,
    pub foreground: Option<Rgba>,
}

impl RichText {
    /// 【贴图】【可见内容】无参数，返回是否存在非分隔线块或表格单元中的非空白文本
    pub fn has_content(&self) -> bool {
        self.blocks.iter().any(|block| match &block.kind {
            BlockKind::Image(_) => true,
            BlockKind::ImageRow(images) => !images.is_empty(),
            BlockKind::Rule => false,
            BlockKind::Table { rows, .. } => rows.iter().any(|row| {
                row.cells.iter().any(|cell| {
                    cell.spans
                        .iter()
                        .any(|span| span.text.chars().any(|ch| !ch.is_whitespace()))
                })
            }),
            _ => block
                .spans
                .iter()
                .any(|span| span.text.chars().any(|ch| !ch.is_whitespace())),
        })
    }
}

/// 【贴图】【段落】记录块类型、对齐方式与片段列表
#[derive(Clone, Debug, PartialEq)]
pub(in crate::cli::pin_clipboard) struct Block {
    pub kind: BlockKind,
    pub align: Align,
    pub background: Option<Rgba>,
    pub spans: Vec<Span>,
}

/// 【贴图】【块类型】描述段落、标题、列表、引用、代码、分隔线、图片及表格语义
#[derive(Clone, Debug, PartialEq)]
pub(in crate::cli::pin_clipboard) enum BlockKind {
    Paragraph,
    Heading(u8),
    Image(ImageBlock),
    ImageRow(Vec<ImageBlock>),
    ListItem {
        depth: usize,
        marker: String,
    },
    Quote {
        depth: usize,
    },
    Code,
    Rule,
    Table {
        rows: Vec<TableRow>,
        style: TableStyle,
    },
}

/// 【贴图】【图片块】记录图像字节（占位图为空）、标签、固有尺寸与可选的显示尺寸
#[derive(Clone, Debug, PartialEq)]
pub(in crate::cli::pin_clipboard) struct ImageBlock {
    pub data: Option<Vec<u8>>,
    pub label: String,
    pub width: u32,
    pub height: u32,
    pub display_width: Option<f32>,
    pub display_height: Option<f32>,
}

/// 【贴图】【表格样式】记录源表格的期望宽度、百分比填充标记、各列宽度与边框颜色
#[derive(Clone, Debug, Default, PartialEq)]
pub(in crate::cli::pin_clipboard) struct TableStyle {
    pub width: Option<f32>,
    pub fill: bool,
    pub columns: Vec<f32>,
    pub border: Option<Rgba>,
}

/// 【贴图】【表格行】记录是否表头、行背景与单元格列表
#[derive(Clone, Debug, Default, PartialEq)]
pub(in crate::cli::pin_clipboard) struct TableRow {
    pub header: bool,
    pub background: Option<Rgba>,
    pub cells: Vec<TableCell>,
}

/// 【贴图】【表格单元】记录片段、对齐、背景与是否表头
#[derive(Clone, Debug, PartialEq)]
pub(in crate::cli::pin_clipboard) struct TableCell {
    pub spans: Vec<Span>,
    pub align: Align,
    pub background: Option<Rgba>,
    pub header: bool,
    pub width: Option<f32>,
}

/// 【贴图】【对齐】描述段落的水平对齐方式
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::cli::pin_clipboard) enum Align {
    #[default]
    Left,
    Center,
    Right,
}

/// 【贴图】【片段】记录文本及其样式
#[derive(Clone, Debug, Default, PartialEq)]
pub(in crate::cli::pin_clipboard) struct Span {
    pub text: String,
    pub style: SpanStyle,
}

/// 【贴图】【片段样式】记录内联文本的字重、斜体、装饰、代码、链接、颜色、字号、衬线与行高
#[derive(Clone, Debug, Default, PartialEq)]
pub(in crate::cli::pin_clipboard) struct SpanStyle {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
    pub overline: bool,
    pub code: bool,
    pub serif: bool,
    pub link: bool,
    pub color: Option<Rgba>,
    pub background: Option<Rgba>,
    pub size: Option<f32>,
    pub line_height: Option<f32>,
}
