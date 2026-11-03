//! 【截图运行时】【工作线程】限制捕获阻塞任务数量，空闲工作线程及时释放

use std::time::Duration;

/// 【截图运行时】【构建】创建单线程异步运行时，最多四个阻塞线程，空闲二百五十毫秒后释放；返回运行时或系统错误
pub(super) fn build() -> std::io::Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(4)
        .thread_keep_alive(Duration::from_millis(250))
        .build()
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::sync::mpsc;
    use std::time::Duration;

    struct ThreadExit(mpsc::Sender<()>);

    impl Drop for ThreadExit {
        /// 【运行时测试】【线程退出】在线程局部对象释放时发送退出信号；无返回值
        fn drop(&mut self) {
            let _ = self.0.send(());
        }
    }

    thread_local! {
        static EXIT: RefCell<Option<ThreadExit>> = const { RefCell::new(None) };
    }

    /// 【运行时测试】【空闲释放】真实阻塞任务结束后，运行时仍存活时工作线程应在一秒内退出
    #[test]
    fn an_idle_blocking_thread_exits_while_the_runtime_remains_alive() {
        let runtime = super::build().unwrap();
        let (sender, receiver) = mpsc::channel();
        runtime.block_on(async {
            tokio::task::spawn_blocking(move || {
                EXIT.with(|value| *value.borrow_mut() = Some(ThreadExit(sender)));
            })
            .await
            .unwrap();
        });
        receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("the idle blocking thread must exit without shutting down the runtime");
        runtime.block_on(async {});
    }
}
