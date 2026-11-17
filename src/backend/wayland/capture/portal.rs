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
use std::sync::Mutex;

/// 【Portal捕获】【对外授权】按 include_cursor 完成选择来源和 Start，返回可供截图提前挂载的会话。
///
/// 入口层在读取显示器列表之前调用，拿到的会话还不能直接取帧：它的流要等输出列表确定后才绑定。
pub async fn authorize(
    include_cursor: bool,
) -> crate::backend::capture_session::Result<session::PortalCapture> {
    session::PortalCapture::authorize(include_cursor).await
}

pub struct PortalMethod {
    pub include_cursor: bool,
    /// 【Portal捕获】【提前授权】截图入口在读取显示器列表前就发起的选择来源会话。
    /// 用户先开机再插显示器时，布局变化和授权窗口动画几乎同时结束；
    /// 先授权能保证读到的显示器列表已经是新布局，同时把授权等待并入用户本来就要经历的时间。
    /// 只在单次截图路径使用，长截图仍然自己建立会话。
    prepared: Option<Mutex<Option<session::PortalCapture>>>,
}

impl PortalMethod {
    /// 【Portal捕获】【新后端】按 include_cursor 创建不持有授权的后端，返回可按需授权的捕获器
    pub fn with_cursor(include_cursor: bool) -> Self {
        Self {
            include_cursor,
            prepared: None,
        }
    }

    /// 【Portal捕获】【挂载授权】把已经完成的授权接到本后端，之后第一次截图直接复用，返回自身。
    ///
    /// 授权会话里还带着待绑定的输出，所以真正的流仍要等输出列表确定后才开，
    /// 这样“先授权、后确认显示器”的时序不会把流接到错误的屏幕上。
    pub fn with_prepared(mut self, capture: session::PortalCapture) -> Self {
        self.prepared = Some(Mutex::new(Some(capture)));
        self
    }
}

#[async_trait]
impl CaptureMethod for PortalMethod {
    /// 【Portal捕获】【单次截图】按 outputs 使用挂载的授权或新建授权，读取一帧后关闭会话，返回原始图像
    async fn capture_frame(&self, outputs: &[Output]) -> crate::AppResult<CaptureResult> {
        // 1. 【Portal捕获】【授权取用】提前授权只够用一次，取出后立刻置空，避免第二帧复用一个已经交付过流的会话
        let mut capture = match self
            .prepared
            .as_ref()
            .and_then(|slot| slot.lock().ok().and_then(|mut capture| capture.take()))
        {
            Some(capture) => capture,
            None => session::PortalCapture::authorize(self.include_cursor)
                .await
                .map_err(|error| error as Box<dyn std::error::Error>)?,
        };
        // 2. 【Portal捕获】【绑定输出】挂载的授权在这里才接触输出列表，流按最终布局绑定
        let result = async {
            capture.attach(outputs).await?;
            capture.next_frame().await
        }
        .await;
        if let Err(error) = capture.close().await {
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
