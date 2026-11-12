//! 【截图起始】【界面状态】一次性选择入口、录制状态与只读快捷键提示共用紧凑控件

mod bar;
pub(crate) mod draw;
pub(crate) mod hints;
pub(crate) mod layout;
#[cfg(test)]
mod tests;
mod text;

use crate::features::recording::{Format, Status, status_query::Query};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Control {
    Region,
    Fullscreen,
    Video,
    Gif,
    Pause,
    Stop,
    Background,
}

#[derive(Default)]
pub(crate) struct State {
    pub visible: bool,
    pub checked: bool,
    pub stop_requested: bool,
    pub recording: Option<Format>,
    pub hovered: Option<(usize, Control)>,
    pub pressed: Option<(usize, Control)>,
    pub status: Status,
    pub query: Query,
    pub message: String,
}

impl State {
    /// 【截图起始】【停止防重】无参数，首次活动录制停止返回 true，并立即锁定控制按钮直到会话结束
    pub(crate) fn request_stop(&mut self) -> bool {
        if !self.active() || self.status.phase == "saving" || self.stop_requested {
            return false;
        }
        self.stop_requested = true;
        true
    }

    /// 【截图起始】【活动会话】无参数，返回录制是否仍占用采集或保存资源
    pub(crate) fn active(&self) -> bool {
        matches!(
            self.status.phase.as_str(),
            "countdown" | "starting" | "recording" | "paused" | "saving"
        )
    }

    /// 【截图起始】【入口可用】无参数，返回是否允许开始新的视频或 GIF 会话，已有会话时保持禁用
    pub(crate) fn can_record(&self) -> bool {
        self.checked && !self.status.window_open
    }

    /// 【截图起始】【暂停可用】无参数，返回暂停或继续是否仍然有效，请求停止后不再接受
    pub(crate) fn can_pause(&self) -> bool {
        !self.stop_requested && matches!(self.status.phase.as_str(), "recording" | "paused")
    }

    /// 【截图起始】【停止可用】无参数，返回停止并保存是否仍然有效，导出阶段不接受第二次停止
    pub(crate) fn can_stop(&self) -> bool {
        self.active() && !self.stop_requested && self.status.phase != "saving"
    }

    /// 【截图起始】【提示条可见】无参数，返回是否需要显示录制消息条，与顶部栏状态互不重复
    pub(crate) fn message_visible(&self) -> bool {
        !self.active() && (self.status.window_open || !self.message.is_empty())
    }
}
