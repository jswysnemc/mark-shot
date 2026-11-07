//! 【框选翻译】【语言缓存】一次翻译保留原图和识别结果，切换目标复用已完成译文

use super::pixels::Region;
use crate::features::image_translation::{SourceBlock, Token, TranslatedBlock, model, paragraphs};
use std::collections::HashMap;
use tiny_skia::Pixmap;

pub struct Translation {
    pub pixels: Pixmap,
    pub blocks: Vec<TranslatedBlock>,
    pub text: String,
}

pub struct Session {
    pub region: Region,
    pub tokens: Vec<Token>,
    pub source: Vec<SourceBlock>,
    pub translations: HashMap<String, Translation>,
    pub language: String,
    pub showing_translation: bool,
}

impl Session {
    /// 【框选翻译】【会话创建】按 region 保存原图范围，返回空缓存，不初始化识别或服务
    pub fn new(region: Region) -> Self {
        Self {
            region,
            tokens: Vec::new(),
            source: Vec::new(),
            translations: HashMap::new(),
            language: String::new(),
            showing_translation: false,
        }
    }

    /// 【框选翻译】【识别接收】按 text 保存区域内 token 和整段原文，返回是否有可翻译文字
    pub fn recognized(&mut self, text: &crate::ocr::OcrText) -> bool {
        self.tokens = Token::from_ocr(text);
        self.source = paragraphs::group(&self.tokens);
        !self.source.is_empty()
    }

    /// 【框选翻译】【当前结果】返回当前语言缓存，无参数，尚无结果时为空
    pub fn current(&self) -> Option<&Translation> {
        self.translations.get(&self.language)
    }

    /// 【框选翻译】【缓存切换】按 language 选择已有缓存，返回是否命中，失败保留当前语言和显示
    pub fn cached(&mut self, language: &str) -> bool {
        if !self.translations.contains_key(language) {
            return false;
        }
        self.language = language.into();
        self.showing_translation = true;
        true
    }

    /// 【框选翻译】【结果缓存】按 language、pixels 和 blocks 保存非空译文，返回是否更新，空结果不改变原显示
    pub fn complete(
        &mut self,
        language: String,
        pixels: Pixmap,
        blocks: Vec<TranslatedBlock>,
    ) -> bool {
        let text = model::text(&blocks);
        if text.is_empty() {
            return false;
        }
        self.translations.insert(
            language.clone(),
            Translation {
                pixels,
                blocks,
                text,
            },
        );
        self.language = language;
        self.showing_translation = true;
        true
    }
}
