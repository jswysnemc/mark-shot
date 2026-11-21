//! 【贴图】【HTML解析】将剪贴板 HTML 解析为带样式的段落与片段，不执行脚本或外部资源

mod model;
mod parser;
mod tags;
mod token;

#[allow(unused_imports)]
pub(super) use model::{
    Align, Block, BlockKind, ImageBlock, RichText, Span, SpanStyle, TableCell, TableRow, TableStyle,
};
pub(super) use parser::parse;
