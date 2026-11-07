//! 【niri 窗口识别】【协议快照】共用已认证 IPC，在覆盖层出现前读取一次桌面几何

mod geometry;
mod layout;
mod model;
mod panels;
#[cfg(test)]
mod tests;

use super::{Result, Window};
use crate::backend::wayland::niri::ipc::{Client, Peer};
use serde_json::json;
use std::path::Path;
use wayland_client::Connection;

/// 【niri 窗口识别】【桌面读取】按 connection 和 path 读取窗口、工作区、输出与面板，返回活动工作区窗口矩形
pub(super) async fn snapshot(connection: &Connection, path: &Path) -> Result<Vec<Window>> {
    let peer = Peer::wayland(connection)?;
    let mut client = Client::connect(path, peer).await?;
    let outputs: model::Outputs = client.query(&json!("Outputs"), "Outputs").await?;
    let workspaces: Vec<model::Workspace> =
        client.query(&json!("Workspaces"), "Workspaces").await?;
    let windows: Vec<model::Window> = client.query(&json!("Windows"), "Windows").await?;
    let layers: Vec<model::Layer> = client
        .query(&json!("Layers"), "Layers")
        .await
        .unwrap_or_default();
    Ok(geometry::build(
        &outputs,
        &workspaces,
        &windows,
        &layers,
        &layout::load(),
        &panels::load(),
    ))
}
