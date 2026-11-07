//! 【Windows任务】【后台唤醒】内核事件与窗口消息共同等待，不创建通知线程

use crate::backend::windows::native;
use std::io;

pub type Wake = native::Wake;

pub struct Source(Wake);

impl Source {
    /// 【Windows任务】【来源创建】无参数，返回自动复位通知源，不创建窗口或线程
    pub fn new() -> io::Result<Self> {
        native::Wake::new().map(Self)
    }

    /// 【Windows任务】【完成句柄】无参数，返回可跨线程持有的通知句柄，不转移源拥有权
    pub fn wake(&self) -> Wake {
        self.0.clone()
    }

    /// 【Windows任务】【等待句柄】无参数，返回内核事件值，调用方不得关闭
    pub fn handle(&self) -> isize {
        self.0.handle()
    }
}
