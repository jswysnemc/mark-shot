//! 【Linux录制】【独立组件】采集与编码由同一进程完成，仅传递控制信号和文件状态

mod arguments;
mod wlroots;
mod x11;

use super::{Shared, publish};
use crate::{
    config::recording::Recording,
    features::recording::{Command, Target},
};
use nix::{
    sys::signal::{Signal, kill},
    unistd::Pid,
};
use std::{
    path::Path,
    process::Stdio,
    sync::mpsc::{Receiver, RecvTimeoutError},
    time::{Duration, Instant},
};

/// 【Linux录制】【会话执行】按配置和 target 采集到 path，处理有界控制消息，返回组件错误
pub(super) fn run(
    config: &Recording,
    target: &Target,
    path: &Path,
    commands: &Receiver<Command>,
    status: &Shared,
) -> Result<(), String> {
    if std::env::var_os("WAYLAND_DISPLAY").is_some()
        && wlroots::preferred()
        && config.encoder == "auto"
    {
        let mut hardware = config.clone();
        hardware.encoder = "hardware".into();
        match execute(&hardware, target, path, commands, status) {
            Ok(()) => return Ok(()),
            Err(error) if status.lock().unwrap().phase == "starting" => {
                log::info!("【Linux录制】【软件回退】{error}");
                let _ = std::fs::remove_file(path);
                let mut software = config.clone();
                software.encoder = "software".into();
                return execute(&software, target, path, commands, status)
                    .map_err(|fallback| format!("Hardware: {error}\nSoftware: {fallback}"));
            }
            Err(error) => return Err(error),
        }
    }
    execute(config, target, path, commands, status)
}

/// 【Linux录制】【单次尝试】按相同参数执行一个后端，开始写帧后不再静默重启或切换来源
fn execute(
    config: &Recording,
    target: &Target,
    path: &Path,
    commands: &Receiver<Command>,
    status: &Shared,
) -> Result<(), String> {
    let direct = std::env::var_os("WAYLAND_DISPLAY").is_some() && wlroots::preferred();
    let mut process = if direct {
        wlroots::build(config, target, path)?
    } else {
        arguments::build(config, target, path)?
    };
    process
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    // 1. 【Linux录制】【进程归属】父进程异常退出时通知录制组件收尾，不使用全局 pkill
    use std::os::unix::process::CommandExt;
    unsafe {
        process.pre_exec(|| {
            if nix::libc::prctl(nix::libc::PR_SET_PDEATHSIG, nix::libc::SIGINT) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            if nix::libc::getppid() == 1 {
                return Err(std::io::Error::other("recording parent exited"));
            }
            Ok(())
        });
    }
    let mut child = process.spawn().map_err(|error| error.to_string())?;
    let pid = Pid::from_raw(child.id() as i32);
    let (errors, reader) = crate::features::recording::export::stderr(&mut child);
    let mut began = None;
    let mut paused_at = None;
    let mut paused_total = Duration::ZERO;
    let mut stopping = None;
    let mut forced = false;
    let result = loop {
        let now = Instant::now();
        // 2. 【Linux录制】【真实启动】文件有视频数据后才显示录制；门户授权等待不计入时长
        if began.is_none()
            && (errors
                .lock()
                .unwrap()
                .windows(b"MARK_SHOT_RECORDING_READY".len())
                .any(|window| window == b"MARK_SHOT_RECORDING_READY")
                || std::fs::metadata(path).is_ok_and(|file| file.len() > 4096))
        {
            began = Some(now);
        }
        let elapsed = began.map_or(Duration::ZERO, |start| {
            paused_at
                .unwrap_or(now)
                .saturating_duration_since(start)
                .saturating_sub(paused_total)
        });
        if stopping.is_none() && began.is_some() {
            publish(
                status,
                if paused_at.is_some() {
                    "paused"
                } else {
                    "recording"
                },
                if direct {
                    if config.encoder == "hardware" {
                        "wf-recorder · VAAPI"
                    } else {
                        "wf-recorder · software H.264"
                    }
                } else {
                    "GPU Screen Recorder"
                },
                elapsed.as_millis() as u64,
            );
        }
        match child.try_wait() {
            Ok(Some(exit)) => {
                break if !forced && exit.success() && began.is_some() {
                    Ok(())
                } else {
                    Err(format!("录制组件退出 / Recording component exited: {exit}"))
                };
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                break Err(error.to_string());
            }
            _ => {}
        }
        if stopping
            .is_some_and(|start: Instant| now.duration_since(start) > Duration::from_secs(15))
        {
            forced = true;
            let _ = child.kill();
        }
        match commands.recv_timeout(Duration::from_millis(100)) {
            Ok(Command::Stop) | Err(RecvTimeoutError::Disconnected) if stopping.is_none() => {
                stopping = Some(now);
                publish(status, "saving", "", elapsed.as_millis() as u64);
                let _ = kill(pid, Signal::SIGINT);
            }
            Ok(Command::Pause) if began.is_some() && paused_at.is_none() && stopping.is_none() => {
                if kill(pid, Signal::SIGUSR2).is_ok() {
                    paused_at = Some(now);
                }
            }
            Ok(Command::Resume)
                if paused_at.is_some()
                    && stopping.is_none()
                    && kill(pid, Signal::SIGUSR2).is_ok() =>
            {
                paused_total += now.duration_since(paused_at.take().unwrap());
            }
            _ => {}
        }
    };
    let _ = reader.join();
    result.map_err(|error| {
        format!(
            "{error}\n{}",
            String::from_utf8_lossy(&errors.lock().unwrap())
        )
    })
}
