//! 【Windows快捷截图】【进程预热】后台准备一次性待命进程，激活时使用最新配置，截图退出后准备下一次

use super::{actions::Action, standby};
use std::{
    io::{Read, Write},
    os::windows::process::CommandExt,
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};

enum Message {
    Ready(Result<Standby, String>),
    Action(Action),
    Stop,
}

pub(super) struct Worker {
    sender: mpsc::Sender<Message>,
    busy: Arc<AtomicBool>,
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

/// 【Windows快捷截图】【待命准备】无参数，预先加载可执行文件和字体目录，十秒内返回已就绪子进程；失败自动回收资源
fn prepare() -> crate::AppResult<Standby> {
    let mut command = Command::new(std::env::current_exe()?);
    command.arg(standby::ARG);
    crate::config::pass_to(&mut command);
    command
        .creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(crate::logging::open_log_file().map_or_else(Stdio::null, Stdio::from));
    let mut child = Standby(command.spawn()?);
    log::info!("【Windows快捷截图】【待命创建】进程编号={}", child.0.id());
    let mut output = child
        .0
        .stdout
        .take()
        .ok_or("standby output is unavailable")?;
    let (sender, receiver) = mpsc::sync_channel(1);
    // 1. 【Windows快捷截图】【有界就绪】独立管道读取不能阻塞动作线程，超时由资源句柄终止异常子进程
    std::thread::Builder::new()
        .name("capture-ready".into())
        .spawn(move || {
            let mut ready = [0];
            let result = output.read_exact(&mut ready).map(|()| ready == *b"\n");
            let _ = sender.send(result);
        })?;
    if !receiver.recv_timeout(Duration::from_secs(10))?? {
        return Err("capture standby did not become ready".into());
    }
    log::info!("【Windows快捷截图】【待命就绪】进程编号={}", child.0.id());
    Ok(child)
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

/// 【Windows快捷截图】【并行准备】按 sender 在独立线程准备待命进程，返回线程创建结果；就绪等待不阻塞用户动作
fn prepare_async(sender: mpsc::Sender<Message>) -> std::io::Result<()> {
    std::thread::Builder::new()
        .name("capture-prepare".into())
        .spawn(move || {
            let result = prepare().map_err(|error| error.to_string());
            // 1. 【Windows快捷截图】【准备交付】接收端已停止时消息释放本次进程，避免后台退出留下待命资源
            let _ = sender.send(Message::Ready(result));
        })
        .map(|_| ())
}

/// 【Windows快捷截图】【立即启动】按 action 创建独立截图进程并等待结束，返回会话结果；不等待尚未完成的待命准备
fn cold(action: Action) -> crate::AppResult {
    let mut command = Command::new(std::env::current_exe()?);
    command.args(action.arguments().ok_or("invalid capture action")?);
    crate::config::pass_to(&mut command);
    command
        .creation_flags(
            windows_sys::Win32::System::Threading::CREATE_NEW_PROCESS_GROUP
                | windows_sys::Win32::System::Threading::CREATE_NO_WINDOW,
        )
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(crate::logging::open_log_file().map_or_else(Stdio::null, Stdio::from));
    let mut child = Standby(command.spawn()?);
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
        let running = busy.clone();
        let preparation = sender.clone();
        std::thread::Builder::new()
            .name("capture-entry".into())
            .spawn(move || {
                let mut preparing = prepare_async(preparation.clone()).is_ok();
                let mut retry_after_action = false;
                let mut ready = None;
                while let Ok(message) = receiver.recv() {
                    match message {
                        Message::Stop => break,
                        Message::Ready(result) => {
                            preparing = false;
                            match result {
                                Ok(child) => ready = Some(child),
                                Err(error) => {
                                    log::error!("【Windows快捷截图】【待命失败】{error}");
                                    if retry_after_action {
                                        preparing = prepare_async(preparation.clone()).is_ok();
                                    }
                                }
                            }
                            retry_after_action = false;
                        }
                        Message::Action(action) => {
                            // 2. 【Windows快捷截图】【立即响应】已就绪则写入模式，否则直接启动；准备超时不会延迟本次按键
                            let result = ready
                                .take()
                                .map_or_else(|| cold(action), |child| activate(child, action));
                            if let Err(error) = result {
                                log::error!("【Windows快捷截图】【会话执行】{error}");
                            }
                            // 3. 【Windows快捷截图】【下一次准备】保留尚未完成的准备任务，已消费或失败的待命资源只补充一次
                            if !preparing {
                                preparing = prepare_async(preparation.clone()).is_ok();
                            } else {
                                retry_after_action = true;
                            }
                            running.store(false, Ordering::Release);
                        }
                    }
                }
            })?;
        Ok(Self { sender, busy })
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
    /// 【Windows快捷截图】【服务停止】无参数，停止动作线程；就绪资源和后续准备结果由所属句柄回收，无返回值
    fn drop(&mut self) {
        let _ = self.sender.send(Message::Stop);
    }
}
