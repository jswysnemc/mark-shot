//! 【框选翻译】【识别任务】一次性进程释放模型与全局线程池，完成主动通知，取消等待真实回收

use crate::{
    AppResult,
    backend::task_wake::Wake,
    ocr::{OcrImage, OcrText, models::ModelFiles},
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
    },
    thread::JoinHandle,
};

pub(crate) struct Task {
    cancel: Arc<AtomicBool>,
    result: Receiver<Result<OcrText, String>>,
    worker: Option<JoinHandle<()>>,
}

impl Task {
    /// 【框选翻译】【识别启动】按 image、files 和 wake 启动可取消工作，返回所属任务，不在截图进程初始化推理线程池
    pub fn start(image: OcrImage, files: ModelFiles, wake: Wake) -> AppResult<Self> {
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = cancel.clone();
        let (sender, result) = mpsc::channel();
        let worker = std::thread::Builder::new()
            .name("shot-recognition".into())
            .spawn(move || {
                let output = crate::ocr::worker::recognize(image, files, &flag);
                let _ = sender.send(output);
                wake.notify();
            })?;
        Ok(Self {
            cancel,
            result,
            worker: Some(worker),
        })
    }

    /// 【框选翻译】【识别结果】返回一次已完成结果，无参数，工作进程已回收才交付文本
    pub fn take_result(&mut self) -> Option<Result<OcrText, String>> {
        match self.result.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                Some(Err("recognition worker disconnected".into()))
            }
        }
    }

    /// 【框选翻译】【取消识别】设置当前任务取消标记，无参数，不在界面事件回调阻塞等待
    pub fn cancel(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// 【框选翻译】【识别退出】返回通知工作线程是否真实结束，无参数，结果可用不等同于线程结束
    pub fn is_finished(&self) -> bool {
        self.worker.as_ref().is_none_or(JoinHandle::is_finished)
    }

    /// 【框选翻译】【所属回收】取消并连接工作线程，无参数，仅在窗口退出或确认线程结束时调用
    pub fn shutdown(&mut self) {
        self.cancel();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl Drop for Task {
    /// 【框选翻译】【识别释放】取消当前任务，已经结束时立即连接线程，无参数，不遗留窗口关闭后的识别进程
    fn drop(&mut self) {
        self.cancel();
        if self.is_finished() {
            self.shutdown();
        }
    }
}
