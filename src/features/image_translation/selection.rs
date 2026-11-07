//! 【图片翻译】【字符选择】按原版贴图列宽切分显示行，组合字符不产生独立水平区域

use super::{Token, TranslatedBlock};
use crate::ocr::OcrLine;
use regex::Regex;
use std::sync::OnceLock;

/// 【图片翻译】【字符列宽】按 character 返回原版字符类别列宽，组合、控制、格式及 UTF-16 代理字符不占水平位置
fn weight(character: char) -> f64 {
    static ZERO: OnceLock<Regex> = OnceLock::new();
    let pattern = ZERO.get_or_init(|| Regex::new(r"^[\p{M}\p{Cc}\p{Cf}]$").unwrap());
    if character as u32 > 0xffff || pattern.is_match(character.encode_utf8(&mut [0; 4])) {
        return 0.0;
    }
    if matches!(character as u32,0x1100..=0x115f|0x2329..=0x232a|0x2e80..=0x303e|0x3040..=0xa4cf|0xac00..=0xd7a3|0xf900..=0xfaff|0xfe10..=0xfe19|0xfe30..=0xfe6f|0xff00..=0xff60|0xffe0..=0xffe6)
    {
        2.0
    } else {
        1.0
    }
}

/// 【图片翻译】【译文选择行】按 blocks 返回显示行及原版列宽字符边界，保留省略号，复制全文由段落数据单独提供
pub fn lines(blocks: &[TranslatedBlock]) -> Vec<OcrLine> {
    blocks
        .iter()
        .flat_map(|block| &block.lines)
        .filter_map(|line| {
            let bounds = line.rect.draw_rect()?;
            let weights = line.text.chars().map(weight).collect::<Vec<_>>();
            let total = weights.iter().sum::<f64>();
            let mut char_x = vec![bounds.left()];
            let mut offset = 0.0;
            for value in weights {
                offset += value;
                char_x.push(if total > 0.0 {
                    (line.rect.left() + line.rect.width() * offset / total) as f32
                } else {
                    bounds.right()
                });
            }
            if let Some(last) = char_x.last_mut() {
                *last = bounds.right();
            }
            Some(OcrLine {
                text: line.text.clone(),
                bounds,
                char_x,
            })
        })
        .collect()
}

/// 【图片翻译】【原文选择行】按 tokens 返回交接原文的列宽字符边界，保留实际原图矩形，不运行识别或字体排版
pub fn source_lines(tokens: &[Token]) -> Vec<OcrLine> {
    tokens
        .iter()
        .filter_map(|token| {
            let bounds = token.rect.draw_rect()?;
            let weights = token.text.chars().map(weight).collect::<Vec<_>>();
            let total = weights.iter().sum::<f64>();
            let mut char_x = vec![bounds.left()];
            let mut offset = 0.0;
            for value in weights {
                offset += value;
                char_x.push(if total > 0.0 {
                    (token.rect.left() + token.rect.width() * offset / total) as f32
                } else {
                    bounds.right()
                });
            }
            if let Some(last) = char_x.last_mut() {
                *last = bounds.right();
            }
            Some(OcrLine {
                text: token.text.clone(),
                bounds,
                char_x,
            })
        })
        .collect()
}
