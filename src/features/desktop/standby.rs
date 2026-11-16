//! 【Windows快捷截图】【待命入口】预热字体后阻塞在父进程管道，收到模式时才初始化一次性截图配置

use std::{
    ffi::OsString,
    io::{Read, Write},
};

pub(crate) const ARG: &str = "--capture-standby";

/// 【Windows快捷截图】【待命执行】按 options 保留本次命令行覆盖，收到一字节截图模式后读取最新配置并运行，返回执行结果
pub(crate) fn run(options: crate::cli::arguments::Arguments) -> crate::AppResult {
    crate::utils::memory::configure();
    // 1. 【Windows快捷截图】【无配置预热】只加载字体描述，不固定配置、热键映射或标注偏好，也不捕获屏幕
    let _ = crate::fonts::system();
    std::io::stdout().write_all(b"\n")?;
    std::io::stdout().flush()?;
    let mut mode = [0];
    match std::io::stdin().read_exact(&mut mode) {
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(()),
        result => result?,
    }
    if mode[0] > 1 {
        return Err("invalid capture standby mode".into());
    }
    // 2. 【Windows快捷截图】【最新配置】本进程只进入一次普通截图入口，文件中的设置和保存偏好均在激活后读取
    let mut arguments: Vec<OsString> = vec![
        if mode[0] == 1 {
            "--fullscreen"
        } else {
            "--capture"
        }
        .into(),
    ];
    if let Some(path) = options.config {
        arguments.extend(["--config".into(), path.into_os_string()]);
    }
    if options.no_notify {
        arguments.push("--nonotify".into());
    }
    if let Some(enabled) = options.debug {
        arguments.push(if enabled { "--debug" } else { "--no-debug" }.into());
    }
    if let Some(path) = options.debug_log {
        arguments.extend(["--debug-log".into(), path.into_os_string()]);
    }
    if options.xdg_window {
        arguments.push("--xdg-window".into());
    }
    crate::cli::run(arguments)
}
