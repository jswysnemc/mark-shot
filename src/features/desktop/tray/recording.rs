//! 【系统托盘】【录制状态】将屏幕录制状态映射为本地化菜单，避免展示当前不可执行的操作

use super::{super::Action, menu::Item};
use crate::{features::recording::Status, ui::language::Language};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::features::desktop) enum State {
    #[default]
    Idle,
    Preparing,
    Countdown,
    Starting,
    Recording,
    Paused,
    Saving,
    Unavailable,
}

impl State {
    /// 【系统托盘】【录制快照】按 status 的阶段和窗口占用状态返回菜单状态，不保留时长等高频变化字段
    pub(in crate::features::desktop) fn from_status(status: &Status) -> Self {
        match status.phase.as_str() {
            "countdown" => Self::Countdown,
            "starting" => Self::Starting,
            "recording" => Self::Recording,
            "paused" => Self::Paused,
            "saving" => Self::Saving,
            "idle" | "finished" | "cancelled" | "failed" if status.window_open => Self::Preparing,
            "idle" | "finished" | "cancelled" | "failed" => Self::Idle,
            _ => Self::Unavailable,
        }
    }

    /// 【系统托盘】【录制标题】按 language 返回当前阶段的子菜单标题，活动录制直接显示状态
    pub(super) fn label(self, language: Language) -> &'static str {
        match (self, language == Language::Chinese) {
            (Self::Countdown, true) => "录制倒计时",
            (Self::Countdown, false) => "Recording Countdown",
            (Self::Starting, true) => "录制准备中",
            (Self::Starting, false) => "Starting Recording",
            (Self::Recording, true) => "录制中",
            (Self::Recording, false) => "Recording in Progress",
            (Self::Paused, true) => "录制已暂停",
            (Self::Paused, false) => "Recording Paused",
            (Self::Saving, true) => "录制保存中",
            (Self::Saving, false) => "Saving Recording",
            (_, true) => "屏幕录制",
            (_, false) => "Recording",
        }
    }

    /// 【系统托盘】【录制操作】按 language 返回当前可执行的操作，准备窗口占用、保存及查询失败时只展示禁用提示
    pub(super) fn entries(self, language: Language) -> Vec<Item> {
        let chinese = language == Language::Chinese;
        let item = |cn: &str, en: &str, action, enabled| Item::Action {
            label: if chinese { cn } else { en }.into(),
            action,
            enabled,
        };
        let stop = || item("停止并保存", "Stop and Save", Action::StopRecording, true);
        match self {
            Self::Idle => vec![item("开始录制", "Start Recording", Action::Record, true)],
            Self::Preparing => vec![item(
                "录制窗口已打开",
                "Recording Window Is Open",
                Action::Record,
                false,
            )],
            Self::Countdown | Self::Starting => vec![item(
                "取消录制",
                "Cancel Recording",
                Action::StopRecording,
                true,
            )],
            Self::Recording => vec![
                item("暂停录制", "Pause Recording", Action::PauseRecording, true),
                stop(),
            ],
            Self::Paused => vec![
                item(
                    "继续录制",
                    "Resume Recording",
                    Action::ResumeRecording,
                    true,
                ),
                stop(),
            ],
            Self::Saving => vec![item("正在保存", "Saving", Action::StopRecording, false)],
            Self::Unavailable => vec![item(
                "录制状态暂不可用",
                "Recording Status Unavailable",
                Action::Record,
                false,
            )],
        }
    }
}
