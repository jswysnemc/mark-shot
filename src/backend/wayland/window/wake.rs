//! 【原生窗口】【任务唤醒】通过 eventfd 合并后台通知，空闲时保持阻塞

use smithay_client_toolkit::reexports::calloop::ping::{Ping, PingSource, make_ping};
use std::io;

#[derive(Clone)]
pub struct WindowWake(Kind);

#[derive(Clone)]
enum Kind {
    Wayland(Ping),
    X11(crate::backend::task_wake::Wake),
}

impl WindowWake {
    /// 【原生窗口】【完成通知】唤醒窗口处理已完成任务；合并重复通知，不安排周期重绘
    pub fn notify(&self) {
        match &self.0 {
            Kind::Wayland(ping) => ping.ping(),
            Kind::X11(wake) => wake.notify(),
        }
    }

    /// 【原生窗口测试】【独立通知】创建不连接图形会话的通知句柄；用于验证后台任务生命周期
    #[cfg(test)]
    pub(crate) fn detached() -> io::Result<Self> {
        make_ping().map(|(ping, _)| Self(Kind::Wayland(ping)))
    }

    /// 【X11窗口】【通知适配】按wake创建共享窗口通知句柄，返回不持有图形资源的通知器
    pub(crate) fn x11(wake: crate::backend::task_wake::Wake) -> Self {
        Self(Kind::X11(wake))
    }
}

/// 【原生窗口】【通知源】创建后台可克隆的通知句柄与事件循环源；返回系统创建错误
pub(super) fn source() -> io::Result<(WindowWake, PingSource)> {
    make_ping().map(|(ping, source)| (WindowWake(Kind::Wayland(ping)), source))
}
