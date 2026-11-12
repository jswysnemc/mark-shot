//! 【录制后台】【无窗口等待】界面关闭后保持进程与命令服务，录制结束时通知结果

use super::host::Shared;
use crate::features::recording::Status;
use std::time::Duration;

/// 轮询工作线程是否结束的间隔，暂停、继续与停止由命令服务线程即时处理
const POLL: Duration = Duration::from_millis(100);
/// 录制结束后保留命令服务的时间，让打开中的截图界面读到最终状态
const LINGER: Duration = Duration::from_millis(2500);

/// 【录制后台】【等待结束】按 host 阻塞到工作线程完成采集与导出，返回最终状态
pub(super) fn wait(host: &Shared) -> Status {
    while host.borrow().active() {
        std::thread::sleep(POLL);
    }
    host.borrow().snapshot()
}

/// 【录制后台】【完成通知】按 status 发送保存通知并短暂保留命令服务，无返回值
pub(super) fn finished(status: &Status) {
    let Some(path) = status.output.as_deref().filter(|path| !path.is_empty()) else {
        return;
    };
    log::info!("【屏幕录制】【录制完成】成品已保存: {path}");
    crate::backend::notify::send_blocking(crate::backend::notify::Notice::RecordingSaved(
        path.into(),
    ));
    std::thread::sleep(LINGER);
}
