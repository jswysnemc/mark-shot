//! 【niri】【活动窗口】精确窗口捕获入口及剪贴板副作用边界

mod frame;
mod legacy;
mod target;
#[cfg(test)]
mod tests;

use async_trait::async_trait;
use std::time::Duration;
use wayland_client::Connection;

use crate::{
    backend::CaptureMethod,
    types::{Capture, CaptureResult, Output},
};

use crate::backend::wayland::niri::{Result, ipc};

pub struct NiriWindowMethod {
    connection: Connection,
    include_cursor: bool,
    allow_clipboard: bool,
}

impl NiriWindowMethod {
    /// 【niri】【窗口后端】保存 connection 和指针选项，allow_clipboard 表示本次交付包含复制；返回按需捕获器
    pub fn new(connection: Connection, include_cursor: bool, allow_clipboard: bool) -> Self {
        Self {
            connection,
            include_cursor,
            allow_clipboard,
        }
    }

    /// 【niri】【窗口请求】在统一期限内读取焦点并捕获固定窗口，无参数；返回原生尺寸及输出缩放
    async fn capture(&self) -> Result<Capture> {
        if !self.allow_clipboard {
            return Err("niri window screenshots change the clipboard; this capture requires a copy action (--to c)".into());
        }
        let peer = ipc::Peer::wayland(&self.connection)?;
        let socket = std::env::var_os("NIRI_SOCKET").ok_or("NIRI_SOCKET is not set")?;
        let work = async {
            let mut client = ipc::Client::connect(socket.as_ref(), peer).await?;
            let target = client.target().await?;
            legacy::capture(&mut client, &target, self.include_cursor).await
        };
        tokio::time::timeout(Duration::from_secs(5), work)
            .await
            .map_err(|_| "niri did not finish the active-window capture in time")?
    }
}

#[async_trait]
impl CaptureMethod for NiriWindowMethod {
    /// 【niri】【输出边界】本后端只捕获窗口，拒绝 outputs 对应显示器截图；返回明确错误
    async fn capture_frame(
        &self,
        _outputs: &[Output],
    ) -> std::result::Result<CaptureResult, Box<dyn std::error::Error>> {
        Err("the niri window backend cannot capture outputs".into())
    }

    /// 【niri】【活动窗口捕获】获取精确窗口像素，不创建编辑表面；返回可交付图片或完整错误
    async fn capture_active_window(
        &self,
    ) -> std::result::Result<Capture, Box<dyn std::error::Error>> {
        self.capture().await.map_err(|error| error as _)
    }
}
