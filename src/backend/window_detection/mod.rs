//! 【窗口识别】【单次快照】在截图覆盖层显示前读取合成器窗口，不订阅持续事件

mod geometry;
#[cfg(target_os = "linux")]
mod niri;
#[cfg(any(target_os = "windows", test))]
pub(crate) mod requests;
#[cfg(target_os = "linux")]
mod sway;
#[cfg(target_os = "windows")]
pub(crate) mod windows;

use crate::backend::session::Session as Connection;
#[cfg(target_os = "linux")]
use std::time::Duration;
use tiny_skia::Rect;

#[cfg(target_os = "linux")]
type Result<T> = crate::backend::wayland::niri::Result<T>;

#[derive(Clone, Copy, Debug)]
pub struct Window {
    pub rect: Rect,
    pub z_order: Option<i32>,
    #[cfg(target_os = "windows")]
    pub native_id: usize,
}

/// 【窗口识别】【捕获快照】按 connection 读取同一合成器的可见窗口，返回全局逻辑矩形；不创建表面或长期连接
#[cfg(target_os = "linux")]
pub async fn snapshot(connection: &Connection) -> Vec<Window> {
    let connection = match connection {
        Connection::Wayland(connection) => connection,
        Connection::X11(display) => {
            return crate::backend::x11::windows::snapshot(display).unwrap_or_else(|error| {
                log::info!("【X11窗口识别】【查询失败】{error}");
                Vec::new()
            });
        }
    };
    let work = async {
        if let Some(path) = std::env::var_os("NIRI_SOCKET") {
            niri::snapshot(connection, path.as_ref()).await
        } else if let Some(path) = std::env::var_os("SWAYSOCK") {
            sway::snapshot(connection, path.as_ref()).await
        } else {
            Ok(Vec::new())
        }
    };
    match tokio::time::timeout(Duration::from_millis(500), work).await {
        Ok(Ok(windows)) => windows,
        Ok(Err(error)) => {
            log::info!("【窗口识别】【捕获快照】窗口信息不可用：{error}");
            Vec::new()
        }
        Err(_) => {
            log::info!("【窗口识别】【捕获快照】窗口信息读取超时");
            Vec::new()
        }
    }
}

#[cfg(target_os = "windows")]
/// 【Windows窗口识别】【原生快照】按connection返回映射前可见窗口范围，不订阅持续事件
pub async fn snapshot(_connection: &Connection) -> Vec<Window> {
    windows::snapshot()
}
