//! 【PipeWire】【按需会话】一个可取消的原生线程复用视频连接，帧等待不阻塞异步运行时

mod format;
mod pixels;
mod worker;

use std::{
    os::fd::OwnedFd,
    sync::{Arc, Mutex},
    thread::JoinHandle,
    time::Duration,
};
use tokio::sync::oneshot;

pub struct PipewireFrame {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
}

type FrameResult = Result<PipewireFrame, String>;
const FRAME_TIMEOUT: Duration = Duration::from_secs(5);

struct Request {
    id: u64,
    reply: oneshot::Sender<FrameResult>,
}
enum Command {
    Frame(Request),
    Cancel(u64),
    Stop,
}

pub struct PipewireStream {
    commands: pipewire::channel::Sender<Command>,
    thread: Option<JoinHandle<()>>,
    failure: Arc<Mutex<Option<String>>>,
    next_id: u64,
}

struct PendingRequest {
    commands: pipewire::channel::Sender<Command>,
    id: u64,
}

impl Drop for PendingRequest {
    /// 【PipeWire】【请求取消】取消已结束或中途丢弃的帧等待，不关闭所属会话，无参数、无返回值
    fn drop(&mut self) {
        let _ = self.commands.send(Command::Cancel(self.id));
    }
}

impl PipewireStream {
    /// 【PipeWire】【会话建立】按 node_id 接管独立授权 fd 并建立唯一后台流，返回句柄；设置错误由首帧请求交付
    pub fn new(node_id: u32, fd: OwnedFd) -> std::io::Result<Self> {
        let (commands, receiver) = pipewire::channel::channel();
        let failure = Arc::new(Mutex::new(None));
        let thread_failure = failure.clone();
        let thread = std::thread::Builder::new()
            .name("mark-shot-pipewire".into())
            .spawn(move || {
                if let Err(error) = worker::run(node_id, fd, receiver, thread_failure.clone()) {
                    *thread_failure.lock().unwrap() = Some(error);
                }
            })?;
        Ok(Self {
            commands,
            thread: Some(thread),
            failure,
            next_id: 0,
        })
    }

    /// 【PipeWire】【下一帧】激活同一流并等待本次请求结果，返回紧密RGBA帧；超时或取消会停用流，无参数
    pub async fn next_frame(&mut self) -> Result<PipewireFrame, String> {
        if let Some(error) = self.failure.lock().unwrap().clone() {
            return Err(error);
        }
        if self.thread.as_ref().is_none_or(JoinHandle::is_finished) {
            return Err("PipeWire stream has ended".into());
        }
        self.next_id = self.next_id.wrapping_add(1);
        let id = self.next_id;
        let (reply, response) = oneshot::channel();
        let _pending = PendingRequest {
            commands: self.commands.clone(),
            id,
        };
        self.commands
            .send(Command::Frame(Request { id, reply }))
            .map_err(|_| "PipeWire request channel is unavailable")?;
        match tokio::time::timeout(FRAME_TIMEOUT, response).await {
            Ok(Ok(frame)) => frame,
            Ok(Err(_)) => Err(self
                .failure
                .lock()
                .unwrap()
                .clone()
                .unwrap_or_else(|| "PipeWire worker ended without a frame".into())),
            Err(_) => Err("PipeWire did not deliver a frame in time".into()),
        }
    }

    /// 【PipeWire】【会话关闭】通知原生循环退出并回收唯一线程，重复调用无副作用，无参数，返回线程结果
    pub fn close(&mut self) -> Result<(), String> {
        let Some(thread) = self.thread.take() else {
            return Ok(());
        };
        let _ = self.commands.send(Command::Stop);
        thread
            .join()
            .map_err(|_| "PipeWire capture thread panicked".to_owned())
    }
}

impl Drop for PipewireStream {
    /// 【PipeWire】【资源回收】在异常或调用方遗漏关闭时回收线程，无参数、无返回值
    fn drop(&mut self) {
        if let Err(error) = self.close() {
            log::error!("【PipeWire】【资源回收】{error}");
        }
    }
}
