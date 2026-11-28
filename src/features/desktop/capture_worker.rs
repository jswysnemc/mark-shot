//! 【Windows快捷截图】【进程预热】后台准备一次性待命进程，激活时使用最新配置，截图退出后准备下一次

use super::{actions::Action, standby};
use crate::utils::windows_process::{self, Child, Stream, Streams};
use std::{
    io::{Read, Write},
    process::Command,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};

enum Message {
    Ready(Result<(), String>),
    Action(Action),
    Stop,
}

type StandbySlot = Arc<Mutex<Option<Standby>>>;

pub(super) struct Worker {
    sender: mpsc::Sender<Message>,
    busy: Arc<AtomicBool>,
    stopping: Arc<AtomicBool>,
    standby: StandbySlot,
    thread: Option<std::thread::JoinHandle<()>>,
}

struct Standby(Child);

impl Drop for Standby {
    /// 【Windows快捷截图】【进程回收】无参数，退出及异常路径回收仍存活的待命进程与管道，无返回值
    fn drop(&mut self) {
        if !matches!(self.0.try_wait(), Ok(Some(_))) {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}

/// 【Windows快捷截图】【待命登记】按 stopping 与 slot 登记新创建的待命子进程；若服务已停止则立即销毁并返回停止错误
fn register_standby(stopping: &AtomicBool, slot: &StandbySlot, child: Standby) -> crate::AppResult {
    let previous = {
        let mut guard = slot.lock().unwrap_or_else(|error| error.into_inner());
        if stopping.load(Ordering::Acquire) {
            drop(guard);
            drop(child);
            return Err("capture worker stopping".into());
        }
        guard.replace(child)
    };
    drop(previous);
    Ok(())
}

/// 【Windows快捷截图】【待命清理】按 slot 立即终止并回收当前挂起或就绪的待命子进程，无返回值
fn clear_standby(slot: &StandbySlot) {
    let existing = slot
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .take();
    drop(existing);
}

/// 【Windows快捷截图】【待命准备】按 stopping 与 slot 预先加载可执行文件和字体目录，十秒内返回就绪结果；退出时可立即中断并回收子进程
fn prepare(stopping: &AtomicBool, slot: &StandbySlot) -> crate::AppResult {
    if stopping.load(Ordering::Acquire) {
        return Err("capture worker stopping".into());
    }
    let mut command = Command::new(std::env::current_exe()?);
    command.arg(standby::ARG);
    crate::config::pass_to(&mut command);
    let mut child = Standby(windows_process::spawn(
        &command,
        windows_sys::Win32::System::Threading::CREATE_NO_WINDOW,
        Streams {
            stdin: Stream::Piped,
            stdout: Stream::Piped,
            stderr: crate::logging::open_log_file(),
        },
    )?);
    let pid = child.0.id();
    log::info!("【Windows快捷截图】【待命创建】进程编号={pid}");
    let mut output = child
        .0
        .stdout
        .take()
        .ok_or("standby output is unavailable")?;
    register_standby(stopping, slot, child)?;
    let (sender, receiver) = mpsc::sync_channel(1);
    // 1. 【Windows快捷截图】【有界就绪】独立管道读取不能阻塞动作线程，超时或退出时销毁待命进程立即解除读取阻塞
    std::thread::Builder::new()
        .name("capture-ready".into())
        .spawn(move || {
            let mut ready = [0];
            let result = output.read_exact(&mut ready).map(|()| ready == *b"\n");
            let _ = sender.send(result);
        })?;
    let ready = match receiver.recv_timeout(Duration::from_secs(10)) {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => {
            clear_standby(slot);
            return Err(error.into());
        }
        Err(error) => {
            clear_standby(slot);
            return Err(error.into());
        }
    };
    if !ready || stopping.load(Ordering::Acquire) {
        clear_standby(slot);
        return Err("capture standby did not become ready".into());
    }
    log::info!("【Windows快捷截图】【待命就绪】进程编号={pid}");
    Ok(())
}

/// 【Windows快捷截图】【单次激活】按 child 与 action 交付前台权限及截图模式，返回会话结果；每个待命进程只执行一次截图
fn activate(mut child: Standby, action: Action) -> crate::AppResult {
    // 1. 【Windows快捷截图】【焦点交接】接收热键的进程授权待命进程激活窗口，避免前台锁延迟或拒绝焦点
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::AllowSetForegroundWindow(child.0.id());
    }
    child
        .0
        .stdin
        .take()
        .ok_or("standby input is unavailable")?
        .write_all(&[u8::from(action == Action::Fullscreen)])?;
    let status = child.0.wait()?;
    if !status.success() {
        return Err(format!("capture process exited with {status}").into());
    }
    Ok(())
}

/// 【Windows快捷截图】【并行准备】按 sender、stopping 与 slot 在独立线程准备待命进程，返回线程句柄或创建错误；就绪等待不阻塞用户动作
fn prepare_async(
    sender: mpsc::Sender<Message>,
    stopping: Arc<AtomicBool>,
    slot: StandbySlot,
) -> std::io::Result<std::thread::JoinHandle<()>> {
    std::thread::Builder::new()
        .name("capture-prepare".into())
        .spawn(move || {
            let result = prepare(&stopping, &slot).map_err(|error| error.to_string());
            // 1. 【Windows快捷截图】【准备交付】接收端已停止时同步清理待命进程，避免后台退出留下待命资源
            if stopping.load(Ordering::Acquire) || sender.send(Message::Ready(result)).is_err() {
                clear_standby(&slot);
            }
        })
}

/// 【Windows快捷截图】【立即启动】按 action 创建独立截图进程并等待结束，返回会话结果；不等待尚未完成的待命准备
fn cold(action: Action) -> crate::AppResult {
    let mut command = Command::new(std::env::current_exe()?);
    command.args(action.arguments().ok_or("invalid capture action")?);
    crate::config::pass_to(&mut command);
    let mut child = Standby(windows_process::spawn(
        &command,
        windows_sys::Win32::System::Threading::CREATE_NEW_PROCESS_GROUP
            | windows_sys::Win32::System::Threading::CREATE_NO_WINDOW,
        Streams {
            stdin: Stream::Null,
            stdout: Stream::Null,
            stderr: crate::logging::open_log_file(),
        },
    )?);
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::AllowSetForegroundWindow(child.0.id());
    }
    let status = child.0.wait()?;
    if !status.success() {
        return Err(format!("capture process exited with {status}").into());
    }
    Ok(())
}

impl Worker {
    /// 【Windows快捷截图】【线程准备】无参数，创建动作线程及独立准备线程，返回服务或创建错误；动作从不等待待命就绪
    pub fn new() -> crate::AppResult<Self> {
        let (sender, receiver) = mpsc::channel::<Message>();
        let busy = Arc::new(AtomicBool::new(false));
        let stopping = Arc::new(AtomicBool::new(false));
        let standby: StandbySlot = Arc::new(Mutex::new(None));
        let running = busy.clone();
        let preparation = sender.clone();
        let worker_stopping = stopping.clone();
        let worker_standby = standby.clone();
        let thread = std::thread::Builder::new()
            .name("capture-entry".into())
            .spawn(move || {
                let mut prepare_thread = prepare_async(
                    preparation.clone(),
                    worker_stopping.clone(),
                    worker_standby.clone(),
                )
                .ok();
                let mut retry_after_action = false;
                let mut ready = false;
                while let Ok(message) = receiver.recv() {
                    match message {
                        Message::Stop => break,
                        Message::Ready(result) => {
                            if let Some(handle) = prepare_thread.take() {
                                let _ = handle.join();
                            }
                            match result {
                                Ok(()) => ready = true,
                                Err(error) => {
                                    ready = false;
                                    if !worker_stopping.load(Ordering::Acquire) {
                                        log::error!("【Windows快捷截图】【待命失败】{error}");
                                        if retry_after_action {
                                            prepare_thread = prepare_async(
                                                preparation.clone(),
                                                worker_stopping.clone(),
                                                worker_standby.clone(),
                                            )
                                            .ok();
                                        }
                                    }
                                }
                            }
                            retry_after_action = false;
                        }
                        Message::Action(action) => {
                            // 2. 【Windows快捷截图】【立即响应】已就绪则写入模式，否则直接启动；准备超时不会延迟本次按键
                            let child = if ready {
                                ready = false;
                                worker_standby
                                    .lock()
                                    .unwrap_or_else(|error| error.into_inner())
                                    .take()
                            } else {
                                None
                            };
                            let result =
                                child.map_or_else(|| cold(action), |child| activate(child, action));
                            if let Err(error) = result {
                                log::error!("【Windows快捷截图】【会话执行】{error}");
                            }
                            // 3. 【Windows快捷截图】【下一次准备】保留尚未完成的准备任务，已消费或失败的待命资源只补充一次
                            if !worker_stopping.load(Ordering::Acquire) {
                                if prepare_thread.is_none() {
                                    prepare_thread = prepare_async(
                                        preparation.clone(),
                                        worker_stopping.clone(),
                                        worker_standby.clone(),
                                    )
                                    .ok();
                                } else {
                                    retry_after_action = true;
                                }
                            }
                            running.store(false, Ordering::Release);
                        }
                    }
                }
                clear_standby(&worker_standby);
                if let Some(handle) = prepare_thread.take() {
                    let _ = handle.join();
                }
                clear_standby(&worker_standby);
            })?;
        Ok(Self {
            sender,
            busy,
            stopping,
            standby,
            thread: Some(thread),
        })
    }

    /// 【Windows快捷截图】【动作提交】按 action 唤醒动作线程，返回提交结果；当前截图进行时不积压重复启动
    pub fn submit(&self, action: Action) -> crate::AppResult {
        if self.busy.swap(true, Ordering::AcqRel) {
            return Ok(());
        }
        if self.sender.send(Message::Action(action)).is_err() {
            self.busy.store(false, Ordering::Release);
            return Err("Windows capture worker stopped".into());
        }
        Ok(())
    }
}

impl Drop for Worker {
    /// 【Windows快捷截图】【服务停止】无参数，立即终止待命子进程并等待工作线程回收，避免托盘退出后残留进程
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::Release);
        clear_standby(&self.standby);
        let _ = self.sender.send(Message::Stop);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        clear_standby(&self.standby);
    }
}
