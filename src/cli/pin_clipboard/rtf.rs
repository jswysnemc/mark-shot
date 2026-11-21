//! 【贴图】【RTF解析】将剪贴板 RTF 解析为现有富文本模型，不执行嵌入对象或外部资源

mod encoding;
mod lexer;
mod parser;
mod table;
#[cfg(test)]
mod tests;

use super::html::RichText;

/// 【贴图】【RTF入口】按 text 解析 RTF 并返回富文本；无法识别时返回空内容
pub(super) fn parse(text: &str) -> RichText {
    parser::Parser::new().parse(text)
}
