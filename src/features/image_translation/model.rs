//! 【图片翻译】【段落数据】原图坐标、全文和绘制行分别保存，截断显示不丢失复制全文

use super::BoxRect;
use crate::theme::Rgba;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Token {
    pub text: String,
    #[serde(rename = "box")]
    pub rect: BoxRect,
    pub line: i32,
    pub index: i32,
    pub confidence: f64,
}

impl Token {
    /// 【图片翻译】【识别适配】按 text 把原生 OCR 行转成图片内 token，返回独立结果，不修改字形边界或调用推理
    pub fn from_ocr(text: &crate::ocr::OcrText) -> Vec<Self> {
        text.lines
            .iter()
            .enumerate()
            .map(|(line, value)| Self {
                text: value.text.clone(),
                rect: BoxRect::new(
                    value.bounds.left() as f64,
                    value.bounds.top() as f64,
                    value.bounds.width() as f64,
                    value.bounds.height() as f64,
                ),
                line: line as i32,
                index: 0,
                confidence: 1.0,
            })
            .collect()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SourceBlock {
    pub text: String,
    pub rect: BoxRect,
    pub line_height: f64,
    pub line_pitch: f64,
    pub line_boxes: Vec<BoxRect>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TranslatedLine {
    pub text: String,
    pub rect: BoxRect,
    pub baseline: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TranslatedBlock {
    pub source: BoxRect,
    pub plate: BoxRect,
    pub background: Rgba,
    pub foreground: Rgba,
    pub font_size: f32,
    pub text: String,
    pub lines: Vec<TranslatedLine>,
}

/// 【图片翻译】【全文复制】按 blocks 合并完整译文，段间换行，保留显示截断前内容，不读取剪贴板
pub fn text(blocks: &[TranslatedBlock]) -> String {
    blocks
        .iter()
        .map(|block| block.text.trim())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// 【图片翻译】【译文选择】按 blocks 返回可选择的实际显示行 token，行号连续，跳过空文字和无效框
pub fn line_tokens(blocks: &[TranslatedBlock]) -> Vec<Token> {
    blocks
        .iter()
        .flat_map(|block| &block.lines)
        .filter(|line| !line.text.trim().is_empty() && line.rect.valid())
        .enumerate()
        .map(|(index, line)| Token {
            text: line.text.clone(),
            rect: line.rect,
            line: index as i32,
            index: 0,
            confidence: 1.0,
        })
        .collect()
}

/// 【图片翻译】【贴图交接】按 offset、bounds 平移 blocks，仅丢弃底板完全在范围外的段落，不裁掉全文或行矩形
pub fn offset_blocks(
    blocks: &[TranslatedBlock],
    offset: (f64, f64),
    bounds: BoxRect,
) -> Vec<TranslatedBlock> {
    blocks
        .iter()
        .filter_map(|block| {
            let mut result = block.clone();
            result.source = result.source.shifted(offset);
            result.plate = result.plate.shifted(offset);
            if !result.plate.intersect(bounds).valid() {
                return None;
            }
            for line in &mut result.lines {
                line.rect = line.rect.shifted(offset);
                line.baseline += offset.1;
            }
            Some(result)
        })
        .collect()
}
