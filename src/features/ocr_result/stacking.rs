//! 【OCR 结果窗口】【独立置顶】仅保存本窗口开关，保存失败保留原状态并使用原版限时页脚反馈

use super::window::ResultWindow;
use crate::backend::window::WindowAction;
use std::{
    io,
    time::{Duration, Instant},
};

#[cfg(test)]
mod tests;

pub(super) struct State {
    pub active: bool,
    pub supported: bool,
    pub deadline: Option<Instant>,
}

impl State {
    /// 【OCR 结果窗口】【置顶初始化】读取独立开关，返回状态；读取失败只记录错误，不继承贴图开关，无参数
    pub fn new() -> Self {
        let active = match crate::config::ocr_result_window::current() {
            Ok(value) => value.always_on_top,
            Err(error) => {
                log::error!("【OCR 结果窗口】【置顶读取】无法读取设置: {error}");
                false
            }
        };
        Self {
            active,
            supported: true,
            deadline: None,
        }
    }
}

/// 【OCR 结果窗口】【置顶切换】为 window 保存并切换开关，返回重绘；协议角色由原生运行时按真实能力同步
pub(super) fn toggle(window: &mut ResultWindow) -> WindowAction {
    change(window, crate::config::ocr_result_window::save)
}

/// 【OCR 结果窗口】【保存交付】按 window 当前开关执行 save，成功才改变按钮与角色请求，失败保持正文、任务及开关，返回窗口动作
fn change(window: &mut ResultWindow, save: impl FnOnce(bool) -> io::Result<()>) -> WindowAction {
    if !window.stacking.supported {
        return WindowAction::None;
    }
    let next = !window.stacking.active;
    match save(next) {
        Ok(()) => {
            window.stacking.active = next;
            log::info!("【OCR 结果窗口】【置顶保存】置顶={next}");
        }
        Err(error) => {
            log::error!("【OCR 结果窗口】【置顶保存】保存失败: {error}");
            window.status = Some("Failed to save settings".into());
            window.stacking.deadline = Some(Instant::now() + Duration::from_millis(2500));
        }
    }
    WindowAction::Redraw
}

/// 【OCR 结果窗口】【置顶反馈到期】在 now 消费 window 的单次期限，返回是否改变当前页脚，不能清除后来其他业务的不同提示
pub(super) fn expire(window: &mut ResultWindow, now: Instant) -> bool {
    if window
        .stacking
        .deadline
        .is_none_or(|deadline| now < deadline)
    {
        return false;
    }
    window.stacking.deadline = None;
    if window.status.as_deref() == Some("Failed to save settings") {
        window.status = None;
        true
    } else {
        false
    }
}
