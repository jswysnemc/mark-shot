//! 【Portal捕获】【入口】普通截图与长截图共用有明确生命周期的授权会话

mod session;
mod token;

use crate::{
    backend::{
        CaptureMethod,
        capture_session::{self, CaptureSession},
    },
    types::{CaptureResult, Output},
};
use async_trait::async_trait;

pub struct PortalMethod {
    pub include_cursor: bool,
    connection: Option<wayland_client::Connection>,
}

impl PortalMethod {
    /// 【Portal捕获】【连接构造】按 connection 与 include_cursor 创建可在授权后核对最新显示器布局的捕获器，返回后端实例
    pub fn new(connection: wayland_client::Connection, include_cursor: bool) -> Self {
        Self {
            include_cursor,
            connection: Some(connection),
        }
    }

    /// 【Portal捕获】【独立构造】按 include_cursor 创建不持有原生连接的捕获器，返回后端实例
    pub fn with_cursor(include_cursor: bool) -> Self {
        Self {
            include_cursor,
            connection: None,
        }
    }
}

#[async_trait]
impl CaptureMethod for PortalMethod {
    /// 【Portal捕获】【单次截图】按 outputs 建立授权并读取一帧，结束前关闭流和会话，返回原始图像
    async fn capture_frame(&self, outputs: &[Output]) -> crate::AppResult<CaptureResult> {
        let mut session =
            session::PortalCapture::open(outputs, self.connection.as_ref(), self.include_cursor)
                .await
                .map_err(|error| error as Box<dyn std::error::Error>)?;
        let result = session.next_frame().await;
        if let Err(error) = session.close().await {
            log::error!("【Portal捕获】【单次关闭】{error}");
        }
        result.map_err(|error| error as Box<dyn std::error::Error>)
    }

    /// 【Portal捕获】【持续会话】按 outputs 建立可复用授权与视频流，返回持续捕获接口，不在每一帧重开会话
    async fn open_session(
        &self,
        outputs: &[Output],
    ) -> capture_session::Result<Box<dyn CaptureSession>> {
        Ok(Box::new(
            session::PortalCapture::open(outputs, self.connection.as_ref(), self.include_cursor)
                .await?,
        ))
    }
}
