//! 【录制控制面板】【交互状态】等待真实状态确认控制命令，避免重复点击与无变化重绘

use super::layout::Control;
use crate::features::recording::Status;
use std::time::{Duration, Instant};

const COMMAND_TIMEOUT: Duration = Duration::from_secs(3);

pub(super) struct Model {
    pub status: Status,
    pub error: Option<String>,
    pending: Option<(&'static str, Instant)>,
}

impl Model {
    /// 【录制控制面板】【状态创建】按 status 返回没有待处理命令或错误的界面状态
    pub(super) fn new(status: Status) -> Self {
        Self {
            status,
            error: None,
            pending: None,
        }
    }

    /// 【录制控制面板】【操作可用】按 control 返回是否接受操作，隐藏始终可用，保存期间禁止再次停止
    pub(super) fn enabled(&self, control: Control) -> bool {
        if control == Control::Hide {
            return true;
        }
        if self.pending.is_some() {
            return false;
        }
        match control {
            Control::Toggle => matches!(self.status.phase.as_str(), "recording" | "paused"),
            Control::Stop => matches!(
                self.status.phase.as_str(),
                "starting" | "countdown" | "recording" | "paused"
            ),
            Control::Hide => true,
        }
    }

    /// 【录制控制面板】【命令意图】按 control 与 now 返回待发送命令并锁定重复操作；隐藏或不可用按钮返回空值
    pub(super) fn request(&mut self, control: Control, now: Instant) -> Option<&'static str> {
        if !self.enabled(control) {
            return None;
        }
        let command = match control {
            Control::Toggle if self.status.phase == "paused" => "resume",
            Control::Toggle => "pause",
            Control::Stop => "stop",
            Control::Hide => return None,
        };
        self.pending = Some((command, now + COMMAND_TIMEOUT));
        self.error = None;
        Some(command)
    }

    /// 【录制控制面板】【交付错误】按 error 保存请求失败原因并恢复按钮，无返回值
    pub(super) fn failed(&mut self, error: String) {
        self.error = Some(error);
        self.pending = None;
    }

    /// 【录制控制面板】【状态更新】按 status 与 now 更新已显示秒数和命令确认，返回是否需要重绘
    pub(super) fn update(&mut self, status: Status, now: Instant) -> bool {
        let mut changed = self.status.phase != status.phase
            || self.status.elapsed_ms / 1000 != status.elapsed_ms / 1000;
        if let Some((command, deadline)) = self.pending {
            let confirmed = match command {
                "pause" => status.phase == "paused",
                "resume" => status.phase == "recording",
                "stop" => matches!(
                    status.phase.as_str(),
                    "saving" | "finished" | "cancelled" | "failed"
                ),
                _ => false,
            };
            // 1. 【录制控制面板】【状态确认】服务的即时回复不代表后端已执行，等待实际阶段变化
            if confirmed {
                self.pending = None;
                changed = true;
            } else if now >= deadline {
                self.failed("控制请求未完成，请通过截图界面或托盘重试 / Control timed out; retry from capture or tray".into());
                changed = true;
            }
        }
        self.status = status;
        changed
    }

    /// 【录制控制面板】【状态标题】按 chinese 返回当前录制阶段名称，未知阶段使用录制准备提示
    pub(super) fn title(&self, chinese: bool) -> &'static str {
        let labels = match self.status.phase.as_str() {
            "recording" => ("正在录制", "Recording"),
            "paused" => ("已暂停", "Paused"),
            "saving" => ("正在保存", "Saving"),
            "finished" => ("已完成", "Finished"),
            "failed" => ("录制失败", "Failed"),
            "cancelled" => ("已取消", "Cancelled"),
            _ => ("正在准备", "Starting"),
        };
        if chinese { labels.0 } else { labels.1 }
    }

    /// 【录制控制面板】【时间文本】无参数，返回录制有效时长；一小时后显示小时，暂停时间由后端扣除
    pub(super) fn elapsed(&self) -> String {
        let seconds = self.status.elapsed_ms / 1000;
        if seconds >= 3600 {
            format!(
                "{}:{:02}:{:02}",
                seconds / 3600,
                seconds / 60 % 60,
                seconds % 60
            )
        } else {
            format!("{:02}:{:02}", seconds / 60, seconds % 60)
        }
    }
}
