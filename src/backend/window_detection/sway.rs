//! 【Sway 窗口识别】【桌面树】同一合成器的有界树请求与活动工作区提取

use super::{Result, Window, geometry};
use crate::backend::wayland::niri::ipc::Peer;
use serde_json::Value;
use std::path::Path;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
};
use wayland_client::Connection;

const MAGIC: &[u8; 6] = b"i3-ipc";
const TREE: u32 = 4;
const MAX_REPLY: u32 = 1024 * 1024;
#[cfg(test)]
mod tests;

/// 【Sway 窗口识别】【协议请求】按 stream 读取有界桌面树，返回结构化快照，拒绝错误帧头和回复类型
async fn tree(stream: &mut UnixStream) -> Result<Value> {
    let mut header = [0u8; 14];
    header[..6].copy_from_slice(MAGIC);
    header[10..].copy_from_slice(&TREE.to_le_bytes());
    stream.write_all(&header).await?;
    stream.read_exact(&mut header).await?;
    let length = u32::from_le_bytes(header[6..10].try_into()?);
    let kind = u32::from_le_bytes(header[10..14].try_into()?);
    if &header[..6] != MAGIC || kind != TREE || length > MAX_REPLY {
        return Err("Sway sent an invalid or oversized tree reply".into());
    }
    let mut bytes = vec![0; length as usize];
    stream.read_exact(&mut bytes).await?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// 【Sway 窗口识别】【真实窗口】按 node 判断是否为原生或 XWayland 窗口，返回命中结果
fn window(node: &Value) -> bool {
    node["app_id"].is_string() || node["window"].as_u64().is_some()
}

/// 【Sway 窗口识别】【逻辑边界】按 node 的桌面矩形读取全局逻辑坐标，返回有效范围
fn rect(node: &Value) -> Option<tiny_skia::Rect> {
    let rect = &node["rect"];
    geometry::rect(
        rect["x"].as_f64()?,
        rect["y"].as_f64()?,
        rect["width"].as_f64()?,
        rect["height"].as_f64()?,
    )
}

/// 【Sway 窗口识别】【焦点顺序】按 node、field 返回子节点，从旧焦点到新焦点排列，最后绘制者具有较高层级
fn ordered<'a>(node: &'a Value, field: &str) -> Vec<&'a Value> {
    let mut children: Vec<_> = node[field].as_array().into_iter().flatten().collect();
    let focus = node["focus"].as_array().map_or(&[][..], Vec::as_slice);
    children.sort_by_key(|child| {
        std::cmp::Reverse(
            focus
                .iter()
                .position(|id| id == &child["id"])
                .unwrap_or(usize::MAX),
        )
    });
    children
}

/// 【Sway 窗口识别】【全屏查找】按 node 与 global 查找全屏窗口，返回真实节点，不把普通选区视为窗口
fn fullscreen(node: &Value, global: bool) -> Option<&Value> {
    let mode = node["fullscreen_mode"].as_u64().unwrap_or_default();
    if window(node) && if global { mode == 2 } else { mode > 0 } {
        return Some(node);
    }
    for field in ["floating_nodes", "nodes"] {
        for child in ordered(node, field).into_iter().rev() {
            if let Some(found) = fullscreen(child, global) {
                return Some(found);
            }
        }
    }
    None
}

/// 【Sway 窗口识别】【窗口提取】按 node、result、depth 和 floating 收集可见窗口，返回值通过 result 追加
fn collect(node: &Value, result: &mut Vec<Window>, depth: usize, floating: bool) {
    if depth >= 64 || result.len() >= 4096 {
        return;
    }
    if window(node) {
        if let Some(rect) = rect(node) {
            let z = result.len() as i32 + if floating { 1 << 20 } else { 0 };
            geometry::append(result, rect, Some(z));
        }
        return;
    }
    // 1. 【Sway 窗口识别】【层级顺序】先处理平铺子节点，再处理浮动窗口，各自保持焦点栈顺序
    for child in ordered(node, "nodes") {
        collect(child, result, depth + 1, floating);
    }
    for child in ordered(node, "floating_nodes") {
        collect(child, result, depth + 1, true);
    }
}

/// 【Sway 窗口识别】【活动桌面】按 tree 提取各输出当前工作区，返回真实窗口边界，其他工作区不参与
fn windows(tree: &Value) -> Vec<Window> {
    let mut result = Vec::new();
    if let Some(window) = fullscreen(tree, true) {
        collect(window, &mut result, 0, true);
        return result;
    }
    for output in ordered(tree, "nodes") {
        if output["type"] != "output" || output["name"] == "__i3" {
            continue;
        }
        let current = output["current_workspace"].as_str();
        for workspace in ordered(output, "nodes") {
            if workspace["type"] != "workspace" {
                continue;
            }
            let visible = current.map_or_else(
                || workspace["visible"].as_bool().unwrap_or(false),
                |name| workspace["name"] == name,
            );
            if !visible {
                continue;
            }
            collect(
                fullscreen(workspace, false).unwrap_or(workspace),
                &mut result,
                0,
                false,
            );
        }
        for window in ordered(output, "floating_nodes") {
            collect(window, &mut result, 0, true);
        }
    }
    result
}

/// 【Sway 窗口识别】【桌面读取】按 connection 和 path 验证合成器身份并请求一次树，返回活动窗口边界
pub(super) async fn snapshot(connection: &Connection, path: &Path) -> Result<Vec<Window>> {
    let mut stream = UnixStream::connect(path).await?;
    if Peer::read(&stream)? != Peer::wayland(connection)? {
        return Err(
            "SWAYSOCK and the Wayland connection belong to different compositor processes".into(),
        );
    }
    Ok(windows(&tree(&mut stream).await?))
}
