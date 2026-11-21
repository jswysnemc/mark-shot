//! 【Windows辅助进程】【独立调用】系统进程独立于父进程生命周期，标准流按调用方指定交接

use crate::utils::windows_process::{self, Stream, Streams};
use std::{
    io,
    os::windows::process::CommandExt,
    process::{Child, Command},
};
use windows_sys::Win32::System::Threading::{CREATE_NEW_PROCESS_GROUP, CREATE_NO_WINDOW};

pub(crate) struct SelfPipes {
    pub stdin: Option<std::fs::File>,
    pub stdout: Option<std::fs::File>,
}

/// 【Windows辅助进程】【自身静默启动】按 command 的参数和环境覆盖创建独立自身进程，capture_stdout 控制就绪管道，返回父端标准流
pub(crate) fn spawn_self(command: &Command, capture_stdout: bool) -> io::Result<SelfPipes> {
    let mut child = windows_process::spawn(
        command,
        CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW,
        Streams {
            stdin: Stream::Piped,
            stdout: if capture_stdout {
                Stream::Piped
            } else {
                Stream::Null
            },
            stderr: crate::logging::open_log_file(),
        },
    )?;
    Ok(SelfPipes {
        stdin: child.stdin.take(),
        stdout: child.stdout.take(),
    })
}

/// 【Windows辅助进程】【外部启动】按 command 创建独立外部进程，返回标准进程句柄，不保留等待线程
pub(crate) fn spawn(mut command: Command) -> io::Result<Child> {
    command.creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW);
    command.spawn()
}
