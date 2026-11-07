//! 【图片上传】【业务契约】内置 HTTP 与自定义命令共用 URL 结果和有限错误，上传不依赖图形框架

mod process;
pub mod request;
pub mod task;

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Output {
    pub url: String,
    pub delete_url: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    Configuration,
    Network,
    Http,
    Protocol,
    Command,
    Timeout,
    Canceled,
    StartFailed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub kind: ErrorKind,
    pub detail: String,
}

impl Error {
    /// 【图片上传】【错误构造】按 kind、detail 返回可交付错误，调用边界不传入响应正文、命令或凭据
    pub(crate) fn new(kind: ErrorKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for Error {
    /// 【图片上传】【错误显示】向 formatter 写入脱敏 detail，返回格式结果，不输出上传图片或服务参数
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;
pub const MAX_IMAGE: usize = 256 * 1024 * 1024;
pub const MAX_RESPONSE: usize = 1024 * 1024;
