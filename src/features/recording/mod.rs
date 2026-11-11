//! 【屏幕录制】【业务入口】原生控制窗口与平台工作线程共用状态契约

mod backend;
mod controller;
mod export;
mod ipc;
mod output;
pub(crate) mod selection;
mod window;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Target {
    pub display: String,
    /// 相对所选输出的物理像素；为空时录制整个输出
    pub crop: Option<[u32; 4]>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Status {
    pub phase: String,
    pub elapsed_ms: u64,
    pub detail: String,
    pub output: Option<String>,
}

impl Default for Status {
    /// 【屏幕录制】【初始状态】无参数，返回没有活动录制的状态
    fn default() -> Self {
        Self {
            phase: "idle".into(),
            elapsed_ms: 0,
            detail: String::new(),
            output: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Command {
    Start,
    Pause,
    Resume,
    Stop,
}

/// 【屏幕录制】【窗口入口】按 target 打开控制窗口，已有会话时报告占用，返回生命周期结果
pub fn run(target: Target) -> crate::AppResult {
    window::run(target)
}

/// 【屏幕录制】【命令入口】按 action 控制同配置会话或查询状态，返回 JSON 到标准输出
pub fn command(action: &str) -> crate::AppResult {
    let status = ipc::request(action)?;
    println!("{}", serde_json::to_string(&status)?);
    Ok(())
}
