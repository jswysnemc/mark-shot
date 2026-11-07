//! 【滚动截图】【帧任务】单个后台请求不阻塞界面，取消与过期结果使用独立请求编号

use crate::backend::{
    capture_session::{CaptureSession, Result},
    task_wake::Wake,
};
use crate::types::CaptureResult;
use std::{sync::mpsc, thread::JoinHandle};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

enum Command {
    Frame(u64),
    Cancel,
    Stop,
}

pub(super) struct Task {
    commands: UnboundedSender<Command>,
    results: mpsc::Receiver<(u64, Result<CaptureResult>)>,
    thread: Option<JoinHandle<Box<dyn CaptureSession>>>,
    pending: Option<u64>,
    sequence: u64,
}

impl Task {
    /// 【滚动截图】【任务创建】用 session 和 wake 创建唯一帧任务线程，返回句柄，不主动请求图像
    pub(super) fn new(session: Box<dyn CaptureSession>, wake: Wake) -> std::io::Result<Self> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let (commands, receiver) = tokio::sync::mpsc::unbounded_channel();
        let (results_tx, results) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("mark-shot-capture".into())
            .spawn(move || runtime.block_on(worker(session, receiver, results_tx, wake)))?;
        Ok(Self {
            commands,
            results,
            thread: Some(thread),
            pending: None,
            sequence: 0,
        })
    }

    /// 【滚动截图】【忙碌状态】返回是否存在当前有效帧请求，无参数，不查询后台线程状态
    pub(super) fn busy(&self) -> bool {
        self.pending.is_some()
    }

    /// 【滚动截图】【帧请求】提交唯一请求并返回发送结果，已有请求时保持不变，无参数
    pub(super) fn request(&mut self) -> std::result::Result<(), String> {
        if self.busy() {
            return Ok(());
        }
        self.sequence = self.sequence.wrapping_add(1);
        self.commands
            .send(Command::Frame(self.sequence))
            .map_err(|_| "the capture worker has ended")?;
        self.pending = Some(self.sequence);
        Ok(())
    }

    /// 【滚动截图】【请求取消】撤销当前请求并通知后台丢弃等待，后续结果不会影响暂停或拖动状态，无参数
    pub(super) fn cancel(&mut self) {
        self.pending = None;
        let _ = self.commands.send(Command::Cancel);
    }

    /// 【滚动截图】【结果接收】返回当前编号的一次结果并清除忙碌状态，过期结果直接丢弃，无参数
    pub(super) fn take(&mut self) -> Option<Result<CaptureResult>> {
        while let Ok((id, result)) = self.results.try_recv() {
            if self.pending == Some(id) {
                self.pending = None;
                return Some(result);
            }
        }
        None
    }

    /// 【滚动截图】【任务结束】取消帧等待并回收线程，返回原捕获会话由界面执行异步关闭，无参数
    pub(super) fn finish(mut self) -> std::result::Result<Box<dyn CaptureSession>, String> {
        self.stop()
    }

    /// 【滚动截图】【线程回收】通知后台结束并交回会话，重复调用返回明确错误，无参数
    fn stop(&mut self) -> std::result::Result<Box<dyn CaptureSession>, String> {
        let thread = self
            .thread
            .take()
            .ok_or("the capture worker is already closed")?;
        let _ = self.commands.send(Command::Stop);
        thread
            .join()
            .map_err(|_| "the capture worker panicked".to_owned())
    }
}

impl Drop for Task {
    /// 【滚动截图】【异常回收】在错误退出时回收帧任务与所属会话，不留下孤立线程，无参数
    fn drop(&mut self) {
        if self.thread.is_some()
            && let Err(error) = self.stop()
        {
            log::error!("【滚动截图】【任务回收】{error}");
        }
    }
}

/// 【滚动截图】【后台循环】用session、commands、results和wake执行单个可取消帧请求，结束时返回原会话
async fn worker(
    mut session: Box<dyn CaptureSession>,
    mut commands: UnboundedReceiver<Command>,
    results: mpsc::Sender<(u64, Result<CaptureResult>)>,
    wake: Wake,
) -> Box<dyn CaptureSession> {
    while let Some(command) = commands.recv().await {
        match command {
            Command::Stop => break,
            Command::Cancel => {}
            Command::Frame(id) => {
                let command = tokio::select! {
                    result = session.next_frame() => { let _ = results.send((id, result)); wake.notify(); None }
                    command = commands.recv() => command,
                };
                if matches!(command, Some(Command::Stop)) {
                    break;
                }
            }
        }
    }
    session
}

#[cfg(test)]
mod tests;
