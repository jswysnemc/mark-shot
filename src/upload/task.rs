//! 【图片上传】【任务生命周期】只在主动上传时创建运行时，完成通知一次，取消与窗口关闭回收所属任务

use super::{Error, ErrorKind, MAX_IMAGE, Output, Result};
use crate::config::upload::Upload;
use std::{sync::mpsc, thread::JoinHandle, time::Duration};
use tokio::sync::oneshot;

pub struct Task {
    cancel: Option<oneshot::Sender<()>>,
    result: Option<mpsc::Receiver<Result<Output>>>,
    worker: Option<JoinHandle<()>>,
}

impl Task {
    /// 【图片上传】【按需开始】按 config、png 和 done 启动任务，返回所属句柄；调用方提供有效环境快照，完成后通知一次
    pub fn start(
        config: Upload,
        png: Vec<u8>,
        done: impl FnOnce() + Send + 'static,
    ) -> Result<Self> {
        if png.is_empty() || png.len() > MAX_IMAGE {
            return Err(Error::new(
                ErrorKind::Configuration,
                "Upload image exceeds limit or is empty",
            ));
        }
        let (cancel, cancellation) = oneshot::channel();
        let (send, result) = mpsc::channel();
        let worker = std::thread::Builder::new().name("mark-shot-upload".into()).spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()
                    .map_err(|_| Error::new(ErrorKind::StartFailed, "Cannot initialize upload task"))?;
                let result = runtime.block_on(async {
                    if config.command.trim().is_empty() {
                        tokio::select! {
                            biased;
                            _ = cancellation => Err(Error::new(ErrorKind::Canceled, "Upload canceled")),
                            result = tokio::time::timeout(Duration::from_millis(config.timeout_ms), super::request::send(&config, &png)) => result.unwrap_or_else(|_| Err(Error::new(ErrorKind::Timeout, "Upload timed out"))),
                        }
                    } else { super::process::run(&config, &png, cancellation).await }
                });
                runtime.shutdown_timeout(Duration::from_millis(100));
                result
            })).unwrap_or_else(|_| Err(Error::new(ErrorKind::StartFailed, "Upload worker failed")));
            let _ = send.send(result);
            done();
        }).map_err(|_| Error::new(ErrorKind::StartFailed, "Cannot start upload worker"))?;
        Ok(Self {
            cancel: Some(cancel),
            result: Some(result),
            worker: Some(worker),
        })
    }

    /// 【图片上传】【主动取消】发送一次取消通知，无参数、无返回值，交互线程不等待网络或外部命令
    pub fn cancel(&mut self) {
        if let Some(cancel) = self.cancel.take() {
            let _ = cancel.send(());
        }
    }

    /// 【图片上传】【结果读取】返回一次性任务结果，无参数；未完成返回空值，不安排轮询或重复交付
    pub fn take_result(&mut self) -> Option<Result<Output>> {
        let result = match self.result.as_ref()?.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return None,
            Err(mpsc::TryRecvError::Disconnected) => Err(Error::new(
                ErrorKind::StartFailed,
                "Upload worker disconnected",
            )),
        };
        self.result = None;
        self.cancel = None;
        Some(result)
    }

    /// 【图片上传】【退出回收】取消并连接本次工作线程，无参数、无返回值，只在任务完成或事件循环退出后使用
    pub fn shutdown(&mut self) {
        self.cancel();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl Drop for Task {
    /// 【图片上传】【所属释放】回收当前任务，无参数、无返回值；异常返回和截图关闭也不能留下上传线程
    fn drop(&mut self) {
        self.shutdown();
    }
}
