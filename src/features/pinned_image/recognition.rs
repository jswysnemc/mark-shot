//! 【贴图】【一次性识别】通过有界匿名文件交接任务，模型与并行线程在子进程退出时释放

use std::{
    io::{self, Seek, SeekFrom},
    os::unix::process::CommandExt,
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

use crate::{
    AppResult,
    ocr::{
        OcrImage, OcrText,
        daemon::protocol::{self, Outgoing, Reply, Request},
        models::ModelFiles,
    },
};

pub const WORKER_ARG: &str = "--pin-recognition-worker";

/// 【贴图】【识别调用】用 files 识别 image；cancel 为真或超过九十秒时终止并回收子进程，返回文字或错误
pub fn recognize(
    image: OcrImage,
    files: ModelFiles,
    cancel: &AtomicBool,
) -> Result<OcrText, String> {
    request(image, files, cancel).map_err(|error| error.to_string())
}

/// 【贴图】【任务交接】将 image、files 写入匿名文件并启动单次任务；cancel 控制等待，返回完整原图坐标结果
fn request(image: OcrImage, files: ModelFiles, cancel: &AtomicBool) -> AppResult<OcrText> {
    if cancel.load(Ordering::Relaxed) {
        return Err("recognition cancelled".into());
    }
    let mut input = tempfile::tempfile()?;
    protocol::write_request(&mut input, Outgoing::Load(&files))?;
    protocol::write_request(&mut input, Outgoing::Recognize(&image))?;
    input.seek(SeekFrom::Start(0))?;
    let mut output = tempfile::tempfile()?;
    let mut command = Command::new(std::env::current_exe()?);
    command.arg(WORKER_ARG);
    crate::config::pass_to(&mut command);
    command
        .stdin(Stdio::from(input))
        .stdout(Stdio::from(output.try_clone()?));
    command.stderr(crate::logging::open_log_file().map_or_else(Stdio::null, Stdio::from));
    let parent = std::process::id() as nix::libc::pid_t;
    // 1. 【贴图】【进程绑定】fork 后只调用系统接口；贴图退出时内核终止识别，避免留下孤立任务
    unsafe {
        command.pre_exec(move || {
            if nix::libc::prctl(nix::libc::PR_SET_PDEATHSIG, nix::libc::SIGKILL) < 0 {
                return Err(io::Error::last_os_error());
            }
            if nix::libc::getppid() != parent {
                return Err(io::Error::from_raw_os_error(nix::libc::ESRCH));
            }
            Ok(())
        });
    }
    let mut child = command.spawn()?;
    let result = (|| {
        let deadline = Instant::now() + Duration::from_secs(90);
        loop {
            if cancel.load(Ordering::Relaxed) || Instant::now() >= deadline {
                return Err("recognition cancelled or timed out".into());
            }
            if let Some(status) = child.try_wait()? {
                if !status.success() {
                    return Err(format!("recognition worker exited with {status}").into());
                }
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        output.seek(SeekFrom::Start(0))?;
        match protocol::read_reply(&mut output)? {
            Reply::Lines(text) => Ok(text),
            Reply::Error(error) => Err(error.into()),
            _ => Err("unexpected recognition worker response".into()),
        }
    })();
    // 2. 【贴图】【进程回收】取消、读取失败和正常完成都回收本次任务，不保留推理线程池
    if result.is_err() {
        let _ = child.kill();
    }
    let _ = child.wait();
    result
}

/// 【贴图】【识别进程入口】args 必须为空；读取一次模型和图像任务，返回有界结果帧，不创建 Wayland 窗口
pub fn cli(args: &[String]) -> AppResult {
    if !args.is_empty() {
        return Err("recognition worker accepts no arguments".into());
    }
    let result = (|| {
        let mut input = std::io::stdin().lock();
        let Request::Load(files) = protocol::read_request(&mut input)? else {
            return Err("recognition worker expected model files".into());
        };
        let Request::Recognize(image) = protocol::read_request(&mut input)? else {
            return Err("recognition worker expected an image".into());
        };
        crate::ocr::default_backend(&files)
            .map_err(|error| -> Box<dyn std::error::Error> { error })?
            .recognize(image, &|| false)
            .map_err(|error| -> Box<dyn std::error::Error> { error })
    })();
    let reply = match result {
        Ok(text) => Reply::Lines(text),
        Err(error) => Reply::Error(error.to_string()),
    };
    protocol::write_reply(&mut std::io::stdout().lock(), &reply)?;
    Ok(())
}
