//! 【原生窗口】【任务唤醒】通过 eventfd 合并后台通知，空闲时保持阻塞

use smithay_client_toolkit::reexports::calloop::ping::{Ping, PingSource, make_ping};
use std::io;

#[derive(Clone)]
pub struct WindowWake {
    ping: Ping,
}

impl WindowWake {
    /// 【原生窗口】【完成通知】唤醒窗口处理已完成任务；合并重复通知，不安排周期重绘
    pub fn notify(&self) {
        self.ping.ping();
    }

    /// 【原生窗口测试】【独立通知】创建不连接图形会话的通知句柄；用于验证后台任务生命周期
    #[cfg(test)]
    pub(crate) fn detached() -> io::Result<Self> {
        make_ping().map(|(ping, _)| Self { ping })
    }
}

/// 【原生窗口】【通知源】创建后台可克隆的通知句柄与事件循环源；返回系统创建错误
pub(super) fn source() -> io::Result<(WindowWake, PingSource)> {
    make_ping().map(|(ping, source)| (WindowWake { ping }, source))
}
