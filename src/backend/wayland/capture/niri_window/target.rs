//! 【niri】【捕获目标】活动窗口稳定身份与所属输出缩放

use super::Result;
use crate::backend::wayland::niri::ipc::Client;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub(super) struct Window {
    pub id: u64,
    pub workspace_id: Option<u64>,
    pub is_focused: bool,
}

#[derive(Deserialize)]
struct Workspace {
    id: u64,
    output: Option<String>,
}

#[derive(Deserialize)]
struct Output {
    logical: Option<Logical>,
}

#[derive(Deserialize)]
struct Logical {
    scale: f64,
}

pub(super) struct Target {
    pub window: Window,
    pub scale: f32,
}

impl Client {
    /// 【niri】【焦点快照】读取活动窗口及其工作区所属输出，无参数；返回稳定窗口 ID 与实际输出比例
    pub(super) async fn target(&mut self) -> Result<Target> {
        let window = self.focused().await?;
        let workspaces: Vec<Workspace> = self.query(&json!("Workspaces"), "Workspaces").await?;
        let outputs: HashMap<String, Output> = self.query(&json!("Outputs"), "Outputs").await?;
        let scale = target_scale(&window, &workspaces, &outputs)?;
        let current = self.focused().await?;
        ensure_same_window(&window, &current)?;
        Ok(Target { window, scale })
    }

    /// 【niri】【活动身份】读取当前真实焦点，无参数；没有窗口焦点时明确返回错误
    pub(super) async fn focused(&mut self) -> Result<Window> {
        let window: Option<Window> = self.query(&json!("FocusedWindow"), "FocusedWindow").await?;
        window
            .filter(|window| window.is_focused)
            .ok_or_else(|| "niri has no focused window".into())
    }
}

/// 【niri】【窗口稳定性】比较 initial 与 current 的窗口和工作区身份，返回是否仍属于同一目标
pub(super) fn ensure_same_window(initial: &Window, current: &Window) -> Result<()> {
    if initial.id != current.id || initial.workspace_id != current.workspace_id {
        return Err("the focused niri window changed during capture".into());
    }
    Ok(())
}

/// 【niri】【输出比例】按 window 的工作区在 workspaces、outputs 中定位缩放；返回有限正比例，不使用当前鼠标输出
fn target_scale(
    window: &Window,
    workspaces: &[Workspace],
    outputs: &HashMap<String, Output>,
) -> Result<f32> {
    let output = workspaces
        .iter()
        .find(|workspace| Some(workspace.id) == window.workspace_id)
        .and_then(|workspace| workspace.output.as_ref())
        .and_then(|name| outputs.get(name))
        .and_then(|output| output.logical.as_ref())
        .ok_or("the focused niri window has no connected output")?;
    if !output.scale.is_finite() || !(0.25..=8.0).contains(&output.scale) {
        return Err("niri sent an invalid window output scale".into());
    }
    Ok(output.scale as f32)
}

#[cfg(test)]
mod tests;
