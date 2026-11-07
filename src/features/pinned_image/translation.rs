//! 【贴图翻译】【显示状态】原文和译文独立存储，任务取消或语言变更不会接受旧结果

use crate::{
    features::image_translation::{SourceBlock, Token, TranslatedBlock, paragraphs},
    ocr::OcrText,
};

pub struct Translation {
    pub source: OcrText,
    pub blocks: Vec<TranslatedBlock>,
    pub target: String,
    pub active: bool,
    pub after_ocr: bool,
    pub pending: Option<u64>,
    pub source_blocks: Vec<SourceBlock>,
}

impl Translation {
    /// 【贴图翻译】【状态创建】按 target 创建未识别、未翻译状态，返回独立对象，不加载模型或服务
    pub fn new(target: String) -> Self {
        Self {
            source: OcrText::default(),
            blocks: Vec::new(),
            target,
            active: false,
            after_ocr: false,
            pending: None,
            source_blocks: Vec::new(),
        }
    }

    /// 【贴图翻译】【段落准备】把当前原文分成完整段落，返回是否存在有效文本，不更改已显示译文
    pub fn prepare(&mut self) -> bool {
        self.source_blocks = paragraphs::group(&Token::from_ocr(&self.source));
        !self.source_blocks.is_empty()
    }

    /// 【贴图翻译】【原图失效】清除原图坐标、任务身份和显示译文，无参数，不修改目标语言
    pub fn clear(&mut self) {
        self.source = OcrText::default();
        self.blocks.clear();
        self.source_blocks.clear();
        self.pending = None;
        self.active = false;
        self.after_ocr = false;
    }

    /// 【贴图翻译】【目标更新】按 target 清除失效缓存并保留识别原文，返回目标是否变化，旧任务身份同时解除
    pub fn set_target(&mut self, target: String) -> bool {
        if target == self.target {
            return false;
        }
        self.target = target;
        self.blocks.clear();
        self.source_blocks.clear();
        self.active = false;
        self.pending = None;
        self.after_ocr = false;
        true
    }

    /// 【贴图翻译】【完成接收】按 id 和 blocks 接受当前任务非空译文，返回是否替换显示，失败或旧身份保留当前画面
    pub fn complete(&mut self, id: u64, blocks: Vec<TranslatedBlock>) -> bool {
        if self.pending != Some(id) {
            return false;
        }
        self.pending = None;
        if blocks.is_empty() {
            return false;
        }
        self.blocks = blocks;
        self.active = true;
        true
    }
}
