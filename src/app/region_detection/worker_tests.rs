//! 【截图分析测试】【关闭响应】慢图像分析不能阻塞选区确认和窗口退出

use super::Worker;
use std::sync::{Arc, atomic::AtomicBool, mpsc};

/// 【截图分析测试】【非阻塞释放】无参数，用等待中的任务模拟慢分析，释放句柄后才能通知退出，验证界面不等待后台
#[test]
fn dropping_pending_analysis_does_not_wait_for_the_worker() {
    let (_sender, result) = mpsc::channel();
    let (release, waiter) = mpsc::channel();
    let (finished, completion) = mpsc::channel();
    let thread = std::thread::spawn(move || {
        let released = waiter
            .recv_timeout(std::time::Duration::from_secs(2))
            .is_ok();
        finished.send(released).unwrap();
    });
    let worker = Worker {
        result,
        thread: Some(thread),
        cancelled: Arc::new(AtomicBool::new(false)),
    };
    drop(worker);
    release
        .send(())
        .expect("UI waited for slow image analysis during cancellation");
    assert!(
        completion
            .recv_timeout(std::time::Duration::from_secs(3))
            .unwrap()
    );
}
