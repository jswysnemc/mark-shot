//! 【图片翻译】【贴图交接】有界元数据与 PNG 分别传输，原图保持原始编码，不把像素写成 JSON 数组

use super::{BoxRect, Token, TranslatedBlock};
use serde::{Deserialize, Serialize};

const MAGIC: &[u8] = b"MS-PIN-TRANSLATION\0";
pub const ARG: &str = "--translation-seed";
pub const LIMIT: usize = 72 * 1024 * 1024 + 32;
const META_LIMIT: usize = 8 * 1024 * 1024;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Seed {
    pub source_tokens: Vec<Token>,
    pub blocks: Vec<TranslatedBlock>,
    pub show_translation: bool,
}

impl Seed {
    /// 【图片翻译】【交接边界】检查当前 source_tokens、blocks 的字形及坐标，返回错误，不分配原图或加载模型
    pub fn validate(&self) -> crate::AppResult {
        if self
            .source_tokens
            .iter()
            .any(|token| !token.rect.valid() || !token.confidence.is_finite())
            || self.blocks.iter().any(|block| {
                !block.source.valid()
                    || !block.plate.valid()
                    || !block.font_size.is_finite()
                    || !(0.25..=3072.0).contains(&block.font_size)
                    || block
                        .lines
                        .iter()
                        .any(|line| !line.rect.valid() || !line.baseline.is_finite())
            })
        {
            return Err("invalid image translation seed geometry".into());
        }
        Ok(())
    }

    /// 【图片翻译】【交接平移】按 offset 和 bounds 平移识别框与译文，返回新 seed，完全在范围外的数据不交给贴图
    pub fn shifted(&self, offset: (f64, f64), bounds: BoxRect) -> Self {
        let source_tokens = self
            .source_tokens
            .iter()
            .filter_map(|token| {
                let mut token = token.clone();
                token.rect = token.rect.shifted(offset);
                token.rect = token.rect.intersect(bounds);
                token.rect.valid().then_some(token)
            })
            .collect();
        Self {
            source_tokens,
            blocks: super::model::offset_blocks(&self.blocks, offset, bounds),
            show_translation: self.show_translation,
        }
    }
}

/// 【图片翻译】【交接编码】按 png 和 seed 返回带有界元数据的内部传输帧，原始 PNG 字节不重编码
pub fn encode(png: &[u8], seed: &Seed) -> crate::AppResult<Vec<u8>> {
    seed.validate()?;
    let metadata = serde_json::to_vec(seed)?;
    if metadata.len() > META_LIMIT || png.len() > 64 * 1024 * 1024 {
        return Err("image translation seed exceeds the byte limit".into());
    }
    let mut result = Vec::with_capacity(MAGIC.len() + 4 + metadata.len() + png.len());
    result.extend_from_slice(MAGIC);
    result.extend_from_slice(&(metadata.len() as u32).to_le_bytes());
    result.extend_from_slice(&metadata);
    result.extend_from_slice(png);
    Ok(result)
}

/// 【图片翻译】【交接解码】按 bytes 返回独立 seed 和借用 PNG，不复制整图，拒绝截断、超限和无效坐标
pub fn decode(bytes: &[u8]) -> crate::AppResult<(Seed, &[u8])> {
    if bytes.len() > LIMIT || !bytes.starts_with(MAGIC) {
        return Err("invalid image translation seed frame".into());
    }
    let header = MAGIC.len() + 4;
    let length = u32::from_le_bytes(
        bytes
            .get(MAGIC.len()..header)
            .ok_or("truncated image translation seed header")?
            .try_into()?,
    ) as usize;
    if length > META_LIMIT {
        return Err("image translation seed metadata exceeds the byte limit".into());
    }
    let metadata = bytes
        .get(header..header + length)
        .ok_or("truncated image translation seed metadata")?;
    let png = &bytes[header + length..];
    if png.is_empty() || png.len() > 64 * 1024 * 1024 {
        return Err("invalid image translation seed image".into());
    }
    let seed: Seed = serde_json::from_slice(metadata)?;
    seed.validate()?;
    Ok((seed, png))
}
