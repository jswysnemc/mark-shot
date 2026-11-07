//! 【翻译服务】【请求快照】纯文本与原图段落共用任务生命周期，图片请求保留真实坐标和段落身份

use super::{Error, ErrorKind, Output, Provider, Result, Segment, source_segments};
use crate::features::image_translation::{BoxRect, SourceBlock, exchange};
use serde_json::{Value, json};

pub enum Request {
    Text(Vec<Segment>),
    Image {
        blocks: Vec<SourceBlock>,
        bounds: BoxRect,
    },
}

impl Request {
    /// 【翻译服务】【文本请求】按 source 返回纯文本行快照，保留空行造成的身份间隔，不修改原文
    pub fn text(source: &str) -> Self {
        Self::Text(source_segments(source))
    }

    /// 【翻译服务】【图片请求】按 blocks 和 image_size 返回真实原图坐标快照，不使用虚拟行框
    pub fn image(blocks: &[SourceBlock], image_size: (u32, u32)) -> Self {
        Self::Image {
            blocks: blocks.to_vec(),
            bounds: BoxRect::new(0.0, 0.0, f64::from(image_size.0), f64::from(image_size.1)),
        }
    }

    /// 【翻译服务】【服务分段】返回当前快照的稳定段落身份和全文，无参数，原生服务返回相同身份
    pub fn segments(&self) -> Vec<Segment> {
        match self {
            Self::Text(segments) => segments.clone(),
            Self::Image { blocks, .. } => blocks
                .iter()
                .enumerate()
                .map(|(index, block)| Segment {
                    id: index as i32,
                    text: block.text.clone(),
                })
                .collect(),
        }
    }

    /// 【翻译服务】【请求边界】按 target 验证非空文本、八 MiB 全文上限及图片范围，返回配置错误，不启动进程
    pub(crate) fn validate(&self, target: &str) -> Result<()> {
        let segments = self.segments();
        let length = segments.iter().try_fold(0_usize, |total, segment| {
            total.checked_add(segment.text.len())
        });
        if length.is_none_or(|length| length > 8 * 1024 * 1024) {
            return Err(Error::new(
                ErrorKind::Configuration,
                "translation source exceeds eight MiB",
            ));
        }
        if target.trim().is_empty()
            || segments.is_empty()
            || segments
                .iter()
                .all(|segment| segment.text.trim().is_empty())
        {
            return Err(Error::new(ErrorKind::Configuration, "No text to translate"));
        }
        if let Self::Image { blocks, bounds } = self
            && (!bounds.valid()
                || blocks.iter().any(|block| {
                    !block.rect.valid()
                        || !block.line_height.is_finite()
                        || !block.line_pitch.is_finite()
                }))
        {
            return Err(Error::new(
                ErrorKind::Configuration,
                "invalid image translation geometry",
            ));
        }
        Ok(())
    }

    /// 【翻译服务】【命令正文】按 target 返回命令交换 JSON；图片段落使用真实坐标，纯文本保留已有协议
    pub(crate) fn input(&self, target: &str) -> Value {
        match self {
            Self::Image { blocks, .. } => exchange::input(blocks, target),
            Self::Text(segments) => {
                json!({"targetLanguage":target,"tokens":segments.iter().map(|segment| json!({"text":segment.text,"box":[0,segment.id as f64*24.0,1000.0,20.0],"line":segment.id,"index":0,"confidence":1.0})).collect::<Vec<_>>()})
            }
        }
    }

    /// 【翻译服务】【图片响应】按 provider 和 root 返回按原图位置恢复的段落译文，缺失段落保持空值，全空返回协议错误
    pub(crate) fn image_output(&self, provider: Provider, root: &Value) -> Option<Result<Output>> {
        let Self::Image { blocks, bounds } = self else {
            return None;
        };
        let tokens = exchange::tokens(root, *bounds);
        let texts = exchange::texts(&tokens, blocks);
        let segments = texts
            .into_iter()
            .enumerate()
            .map(|(index, text)| Segment {
                id: index as i32,
                text,
            })
            .collect();
        let result = Output { provider, segments };
        Some(if result.text().is_empty() {
            Err(Error::new(
                ErrorKind::Protocol,
                root.pointer("/errors/0")
                    .and_then(Value::as_str)
                    .unwrap_or("no translation result"),
            ))
        } else {
            Ok(result)
        })
    }
}
