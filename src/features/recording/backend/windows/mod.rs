//! 【Windows录制】【平台入口】持有运行时并协调首帧兼容回退，不覆盖已经录制的视频

mod audio;
mod capture;
mod converter;
mod device;
mod encoder;
mod gpu;
mod media_type;
mod software;

use super::Shared;
use crate::{
    config::recording::Recording,
    features::recording::{Command, Target},
};
use std::{path::Path, sync::mpsc::Receiver};

/// 【Windows录制】【执行入口】按配置与目标录制到 path，commands 控制暂停停止，status 交付状态，返回错误
pub(super) fn run(
    config: &Recording,
    target: &Target,
    path: &Path,
    commands: &Receiver<Command>,
    status: &Shared,
) -> Result<(), String> {
    let _runtime = device::Runtime::new().map_err(|error| format!("Media Foundation: {error}"))?;
    if config.encoder == "software" {
        return software::run(config, target, path, commands, status, "Software selected");
    }
    let mut retry_first_frame = false;
    let result = gpu::run(
        config,
        target,
        path,
        commands,
        status,
        &mut retry_first_frame,
    );
    match result {
        Err(error) if config.encoder == "auto" && retry_first_frame => {
            // 1. 【Windows录制】【首帧回退】GPU 资源已随调用返回释放，只有尚无成功视频帧时才能重新打开输出文件
            log::info!("【Windows录制】【首帧回退】{error}");
            software::run(config, target, path, commands, status, &error)
                .map_err(|fallback| format!("{error}\nSoftware fallback: {fallback}"))
        }
        result => result,
    }
}

/// 【Windows录制】【系统时间】无参数，返回与 WGC/WASAPI 相同单位的 QPC 百纳秒时间
fn qpc() -> Result<i64, String> {
    use windows::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};
    let mut counter = 0;
    let mut frequency = 0;
    unsafe {
        QueryPerformanceCounter(&mut counter).map_err(|error| error.to_string())?;
        QueryPerformanceFrequency(&mut frequency).map_err(|error| error.to_string())?;
    }
    Ok((i128::from(counter) * 10_000_000 / i128::from(frequency)) as i64)
}
