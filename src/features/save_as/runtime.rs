//! 【截图另存为】【运行时隔离】在独立线程驱动门户请求，支持已有异步运行时的调用方

use std::future::Future;

/// 【截图另存为】【同步等待】接收可跨线程的 future，返回其结果；运行时或工作线程失败时返回错误
pub(super) fn block_on<F>(future: F) -> crate::AppResult<F::Output>
where
    F: Future + Send,
    F::Output: Send,
{
    // 1. 【截图另存为】【线程隔离】门户连接和运行时均在工作线程创建及销毁，避免嵌套进入调用方运行时
    std::thread::scope(|scope| {
        scope
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|error| error.to_string())?;
                Ok::<_, String>(runtime.block_on(future))
            })
            .join()
            .map_err(|_| "file chooser worker panicked")?
            .map_err(Into::into)
    })
}

#[cfg(test)]
mod tests {
    /// 【截图另存为】【单线程回归】无参数，在已有单线程运行时内等待门户任务，返回空值并验证定时器正常执行
    #[tokio::test(flavor = "current_thread")]
    async fn chooser_future_runs_inside_current_thread_runtime() {
        let value = super::block_on(async {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            42
        })
        .unwrap();
        assert_eq!(value, 42);
    }

    /// 【截图另存为】【多线程回归】无参数，在已有多线程运行时内驱动任务，返回空值并验证错误结果保持原样
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn chooser_future_preserves_errors_inside_multithread_runtime() {
        let value = super::block_on(async { Err::<(), _>("portal unavailable") }).unwrap();
        assert_eq!(value, Err("portal unavailable"));
    }
}
