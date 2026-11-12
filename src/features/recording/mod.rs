//! 【屏幕录制】【业务入口】原生控制窗口与平台工作线程共用状态契约

mod backend;
#[cfg(target_os = "linux")]
mod capture;
mod command_delivery;
mod controller;
mod export;
mod ipc;
mod output;
pub(crate) mod selection;
#[cfg(target_os = "linux")]
mod session;
pub(crate) mod status_query;
mod window;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Target {
    pub display: String,
    /// 相对所选输出的物理像素；为空时录制整个输出
    pub crop: Option<[u32; 4]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<Format>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    Video,
    Gif,
}

impl Format {
    /// 【录制入口】【本次格式】按 current 保留视频容器偏好或强制 GIF，返回本次格式且不写配置
    pub(crate) fn resolve(self, current: &str) -> String {
        match self {
            Self::Gif => "gif",
            Self::Video if current == "mkv" => "mkv",
            Self::Video => "mp4",
        }
        .into()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Status {
    #[serde(default)]
    pub window_open: bool,
    pub phase: String,
    pub elapsed_ms: u64,
    pub detail: String,
    pub output: Option<String>,
}

impl Default for Status {
    /// 【屏幕录制】【初始状态】无参数，返回没有活动录制的状态
    fn default() -> Self {
        Self {
            window_open: false,
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
