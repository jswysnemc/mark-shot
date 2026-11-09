//! 【桌面集成】【Windows线程】取消时投递退出消息并等待资源回收，空闲时使用系统消息等待

use tokio::sync::{oneshot, watch};
use windows_sys::Win32::{System::Threading::GetCurrentThreadId, UI::WindowsAndMessaging::*};

struct Stop(u32);
impl Drop for Stop {
    /// 【桌面集成】【异常停止】无参数，异步任务取消时也通知所属消息线程退出
    fn drop(&mut self) {
        unsafe {
            PostThreadMessageW(self.0, WM_QUIT, 0, 0);
        }
    }
}

/// 【桌面集成】【线程就绪】按ready建立消息队列并交付线程编号，返回调用方是否仍等待
pub(super) fn ready(ready: oneshot::Sender<u32>) -> bool {
    unsafe {
        let mut message = std::mem::zeroed();
        PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, PM_NOREMOVE);
        ready.send(GetCurrentThreadId()).is_ok()
    }
}

/// 【桌面集成】【线程托管】按cancelled和worker运行原生服务，返回注册或消息循环错误并保证退出完成
pub(super) async fn run(
    mut cancelled: watch::Receiver<bool>,
    worker: impl FnOnce(oneshot::Sender<u32>) -> Result<(), String> + Send + 'static,
) -> Result<(), String> {
    let (sender, receiver) = oneshot::channel();
    let mut task = tokio::task::spawn_blocking(move || worker(sender));
    let Ok(id) = receiver.await else {
        return task.await.map_err(|error| error.to_string())?;
    };
    let stop = Stop(id);
    if !*cancelled.borrow() {
        tokio::select! {
            _ = cancelled.changed() => {},
            result = &mut task => return result.map_err(|error| error.to_string())?,
        }
    }
    drop(stop);
    task.await.map_err(|error| error.to_string())?
}
