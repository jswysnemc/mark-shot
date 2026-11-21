//! 【RTF解析】【代码段落】合并连续等宽段落

use super::super::html::{Block, BlockKind};
use super::super::spans::append_span;

/// 【RTF解析】【代码段落】按 block 判断是否为全部非空白片段都使用等宽字体的普通段落
fn is_code_paragraph(block: &Block) -> bool {
    if !matches!(block.kind, BlockKind::Paragraph) {
        return false;
    }
    let mut has_text = false;
    for span in &block.spans {
        if span
            .text
            .chars()
            .any(|character| !character.is_whitespace())
        {
            has_text = true;
            if !span.style.code {
                return false;
            }
        }
    }
    has_text
}

/// 【RTF解析】【合并代码】按 blocks 将连续等宽段落合并为代码块，段落之间以换行连接
pub(super) fn merge_code_blocks(blocks: Vec<Block>) -> Vec<Block> {
    let mut result = Vec::with_capacity(blocks.len());
    let mut run: Option<Block> = None;
    for block in blocks {
        if !is_code_paragraph(&block) {
            if let Some(current) = run.take() {
                result.push(current);
            }
            result.push(block);
            continue;
        }
        match run.as_mut() {
            Some(current) => {
                if let Some(style) = current.spans.last().map(|span| span.style.clone()) {
                    append_span(&mut current.spans, "\n", &style);
                }
                for span in &block.spans {
                    append_span(&mut current.spans, &span.text, &span.style);
                }
            }
            None => {
                run = Some(Block {
                    kind: BlockKind::Code,
                    align: block.align,
                    background: block.background,
                    spans: block.spans,
                });
            }
        }
    }
    if let Some(current) = run {
        result.push(current);
    }
    result
}
