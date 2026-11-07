//! 【持续捕获】【会话接口】固定输出、验证首帧并在退出时关闭长生命周期资源

use super::CaptureMethod;
use crate::types::{CaptureResult, Output};
use async_trait::async_trait;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

#[async_trait]
pub trait CaptureSession: Send {
    /// 【持续捕获】【下一帧】返回绑定输出的一帧，不重新请求桌面授权，无参数
    async fn next_frame(&mut self) -> Result<CaptureResult>;

    /// 【持续捕获】【关闭】释放会话和后台线程，重复调用无副作用，无参数，返回清理结果
    async fn close(&mut self) -> Result<()> {
        Ok(())
    }
}

struct Repeated {
    method: Box<dyn CaptureMethod>,
    outputs: Vec<Output>,
}

struct Seeded {
    session: Box<dyn CaptureSession>,
    first: Option<CaptureResult>,
}

/// 【持续捕获】【请求后端】用 method 和 outputs 创建固定目标的原生单帧适配器，返回会话，不申请 Portal 授权
pub(crate) fn repeated(
    method: Box<dyn CaptureMethod>,
    outputs: &[Output],
) -> Box<dyn CaptureSession> {
    Box::new(Repeated {
        method,
        outputs: outputs.to_vec(),
    })
}

/// 【持续捕获】【首帧保留】把已验证的 first 与 session 组合，第一次读取不重复采集，返回同一会话
pub(crate) fn seeded(
    session: Box<dyn CaptureSession>,
    first: CaptureResult,
) -> Box<dyn CaptureSession> {
    Box::new(Seeded {
        session,
        first: Some(first),
    })
}

#[async_trait]
impl CaptureSession for Repeated {
    /// 【持续捕获】【原生请求】从既有 method 请求固定 outputs，返回完整帧，失败转换为可跨线程错误
    async fn next_frame(&mut self) -> Result<CaptureResult> {
        self.method
            .capture_frame(&self.outputs)
            .await
            .map_err(|error| error.to_string().into())
    }
}

#[async_trait]
impl CaptureSession for Seeded {
    /// 【持续捕获】【首帧交付】先交付已经验证的首帧，其后复用同一 session，无参数
    async fn next_frame(&mut self) -> Result<CaptureResult> {
        if let Some(first) = self.first.take() {
            Ok(first)
        } else {
            self.session.next_frame().await
        }
    }

    /// 【持续捕获】【资源关闭】丢弃未消费首帧并关闭底层 session，无参数，返回关闭结果
    async fn close(&mut self) -> Result<()> {
        self.first = None;
        self.session.close().await
    }
}

#[cfg(test)]
mod tests;
