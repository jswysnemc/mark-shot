//! 【翻译服务】【业务契约】保持分段身份和原文快照，网络请求不依赖图形框架

mod http;
pub mod language;
pub(crate) mod models;
mod process;
mod providers;
pub mod request;
pub(crate) mod request_options;
pub mod resolve;
mod response;
pub mod task;

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provider {
    Bilibili,
    Responses,
    OpenAi,
    Gemini,
    Anthropic,
    Tencent,
    Baidu,
    Youdao,
    Command,
    Helper,
}

impl Provider {
    /// 【翻译服务】【服务标识】返回当前服务的稳定配置标识，无参数，不包含凭据或界面文案
    pub fn id(self) -> &'static str {
        match self {
            Self::Bilibili => "bilibili",
            Self::Responses => "openai-responses",
            Self::OpenAi => "openai-compatible",
            Self::Gemini => "gemini",
            Self::Anthropic => "anthropic",
            Self::Tencent => "tencent-tmt",
            Self::Baidu => "baidu-fanyi",
            Self::Youdao => "youdao-nmt",
            Self::Command => "command",
            Self::Helper => "helper",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Segment {
    pub id: i32,
    pub text: String,
}

/// 【翻译服务】【原文分段】按 text 返回非空原文行，空行仍计入身份；每段去除首尾空白，不修改调用方原文
pub fn source_segments(text: &str) -> Vec<Segment> {
    text.trim()
        .split('\n')
        .enumerate()
        .filter_map(|(index, line)| {
            let text = line.trim();
            (!text.is_empty()).then(|| Segment {
                id: index as i32,
                text: text.to_owned(),
            })
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Output {
    pub provider: Provider,
    pub segments: Vec<Segment>,
}

impl Output {
    /// 【翻译服务】【译文合并】返回按输入顺序合并的译文，无参数，保留段内换行，不写回原文
    pub fn text(&self) -> String {
        self.segments
            .iter()
            .map(|segment| segment.text.trim())
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_owned()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    Configuration,
    Network,
    Http,
    Api,
    Protocol,
    Timeout,
    Canceled,
    StartFailed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub kind: ErrorKind,
    pub provider: Option<Provider>,
    pub detail: String,
}

impl Error {
    /// 【翻译服务】【错误构造】按 kind 和 detail 返回可交付错误；调用边界负责移除所有凭据
    pub(crate) fn new(kind: ErrorKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            provider: None,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for Error {
    /// 【翻译服务】【错误显示】把当前 detail 写入 formatter，返回格式结果，不输出请求头或请求正文
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl std::error::Error for Error {}

type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests;
