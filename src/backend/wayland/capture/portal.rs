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
}

#[async_trait]
impl CaptureMethod for PortalMethod {
    /// 【Portal捕获】【单次截图】按 outputs 建立授权并读取一帧，结束前关闭流和会话，返回原始图像
    async fn capture_frame(&self, outputs: &[Output]) -> crate::AppResult<CaptureResult> {
        let mut session = session::PortalCapture::open(outputs, self.include_cursor)
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
            session::PortalCapture::open(outputs, self.include_cursor).await?,
        ))
    }
}
