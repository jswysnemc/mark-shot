//! 【录制后端】【平台边界】原始视频留在平台线程，主界面仅接收有界状态

#[cfg(target_os = "windows")]
mod windows;

use super::{Command, Status, Target};
use crate::config::recording::Recording;
use std::{
    path::Path,
    sync::{Arc, Mutex, mpsc::Receiver},
};

pub type Shared = Arc<Mutex<Status>>;

/// 【录制后端】【执行分发】按配置、目标和工作路径录制，commands 接收控制，status 只保留最新状态
pub fn run(
    config: &Recording,
    target: &Target,
    path: &Path,
    commands: &Receiver<Command>,
    status: &Shared,
) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    return super::session::run(config, target, path, commands, status);
    #[cfg(target_os = "windows")]
    return windows::run(config, target, path, commands, status);
}

/// 【录制后端】【状态发布】按 phase、detail 和 elapsed 更新 status，不排队保留历史状态
pub fn publish(status: &Shared, phase: &str, detail: &str, elapsed: u64) {
    let mut state = status.lock().unwrap_or_else(|error| error.into_inner());
    state.phase = phase.into();
    state.detail = detail.into();
    state.elapsed_ms = elapsed;
}
