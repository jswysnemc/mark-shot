//! 【Windows任务】【完成事件】自动复位内核事件合并通知，句柄最后一次引用释放时关闭

use std::{io, sync::Arc};
use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE},
    System::Threading::{CreateEventW, SetEvent},
};

struct Handle(HANDLE);

// 1. 【Windows任务】【跨线程所有权】内核事件支持跨线程设置与等待，Arc保证最后一个使用者退出后才关闭句柄
unsafe impl Send for Handle {}
unsafe impl Sync for Handle {}

impl Drop for Handle {
    /// 【Windows任务】【事件释放】无参数，关闭最后一个引用拥有的内核事件，不操作窗口，无返回值
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

#[derive(Clone)]
pub struct Wake(Arc<Handle>);

impl Wake {
    /// 【Windows任务】【事件创建】无参数，返回可跨线程持有的自动复位事件，不启动线程
    pub fn new() -> io::Result<Self> {
        let handle = unsafe { CreateEventW(std::ptr::null(), 0, 0, std::ptr::null()) };
        if handle.is_null() {
            Err(super::last_error())
        } else {
            Ok(Self(Arc::new(Handle(handle))))
        }
    }

    /// 【Windows任务测试】【独立唤醒】无参数，返回不依赖窗口的原生完成事件，供共用后台任务测试使用
    #[cfg(test)]
    pub(crate) fn detached() -> io::Result<Self> {
        Self::new()
    }

    /// 【Windows任务】【完成通知】无参数，合并本次完成通知，无返回值，不新增周期期限
    pub fn notify(&self) {
        unsafe {
            SetEvent(self.0.0);
        }
    }

    /// 【Windows任务】【等待句柄】无参数，返回可供消息循环等待的借用句柄值，调用方不得关闭
    pub fn handle(&self) -> isize {
        self.0.0 as isize
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Wait, dispatch};
    use super::*;
    use std::time::Duration;

    /// 【Windows任务测试】【通知合并】无参数，两次完成只唤醒一次，消费后零期限等待保持超时，不产生持续事件，无返回值
    #[test]
    fn completed_work_wakes_once_and_then_returns_to_idle_wait() {
        let wake = Wake::new().unwrap();
        wake.notify();
        wake.notify();
        assert_eq!(
            dispatch(&[wake.handle()], Some(Duration::from_secs(1))).unwrap(),
            Wait::Source(0)
        );
        assert_eq!(
            dispatch(&[wake.handle()], Some(Duration::ZERO)).unwrap(),
            Wait::Timeout
        );
    }
}
