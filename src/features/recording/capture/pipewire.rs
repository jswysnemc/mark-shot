//! 【录制流】【有界交付】视频只保留最新画面，音频最多保留半秒，控制走 PipeWire 原生唤醒

mod buffers;
mod worker;
use std::{
    collections::VecDeque,
    os::fd::OwnedFd,
    sync::{Arc, Condvar, Mutex},
    time::{Duration, Instant},
};

pub(crate) enum Input {
    Video { node: u32, remote: OwnedFd },
    Audio { microphone: bool },
}
pub(super) enum Command {
    Pause(bool, std::sync::mpsc::SyncSender<()>),
    Stop,
}

#[derive(Default)]
pub(crate) struct Data {
    pub pixels: Vec<u8>,
    pub size: (u32, u32),
    pub ready: bool,
    pub audio: VecDeque<(Instant, Vec<f32>)>,
    pub failure: Option<String>,
    pub last_audio: Option<Instant>,
    pub audio_received: bool,
}
pub(super) type Shared = Arc<(Mutex<Data>, Condvar)>;

pub(crate) struct Stream {
    pub data: Shared,
    commands: pipewire::channel::Sender<Command>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Stream {
    /// 【录制流】【持续启动】按 input 与 fps 启动独立协议线程，返回有界数据槽和控制句柄
    pub fn new(input: Input, fps: u32) -> Result<Self, String> {
        let (commands, receiver) = pipewire::channel::channel();
        let data: Shared = Default::default();
        data.0.lock().unwrap().last_audio = Some(Instant::now());
        let shared = data.clone();
        let thread = std::thread::Builder::new()
            .name("recording-pipewire".into())
            .spawn(move || {
                if let Err(error) = worker::run(input, fps, receiver, shared.clone()) {
                    shared.0.lock().unwrap().failure = Some(error);
                    shared.1.notify_all();
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            data,
            commands,
            thread: Some(thread),
        })
    }
    /// 【录制流】【暂停通知】按 paused 同步切换服务器流，等待清空确认后返回，拒绝跨暂停交付缓存
    pub fn pause(&self, paused: bool) -> Result<(), String> {
        let (send, receive) = std::sync::mpsc::sync_channel(1);
        self.commands
            .send(Command::Pause(paused, send))
            .map_err(|_| "PipeWire control closed")?;
        receive
            .recv_timeout(Duration::from_secs(2))
            .map_err(|_| "PipeWire control timed out".into())
    }
    /// 【录制流】【帧交换】按 timeout 等待最新画面，交换复用 pixels 并返回尺寸，不排队保存旧帧
    pub fn video(
        &self,
        pixels: &mut Vec<u8>,
        timeout: Duration,
    ) -> Result<Option<(u32, u32)>, String> {
        let data = self.data.0.lock().unwrap();
        let (mut data, _) = self
            .data
            .1
            .wait_timeout_while(data, timeout, |data| !data.ready && data.failure.is_none())
            .unwrap();
        if let Some(error) = &data.failure {
            return Err(error.clone());
        }
        if !data.ready {
            return Ok(None);
        }
        std::mem::swap(pixels, &mut data.pixels);
        data.ready = false;
        Ok(Some(data.size))
    }
}

impl Drop for Stream {
    /// 【录制流】【线程结束】无参数，唤醒原生事件循环并等待释放流，不依赖轮询或子进程
    fn drop(&mut self) {
        let _ = self.commands.send(Command::Stop);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
