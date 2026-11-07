//! 【Windows辅助进程】【独立调用】系统进程独立于父进程生命周期，标准流按调用方指定交接

use std::{
    io,
    os::windows::process::CommandExt,
    process::{ChildStdin, ChildStdout, Command},
};
use windows_sys::Win32::System::Threading::{CREATE_NEW_PROCESS_GROUP, CREATE_NO_WINDOW};

pub(crate) struct Pipes {
    pub stdin: Option<ChildStdin>,
    pub stdout: Option<ChildStdout>,
}

/// 【Windows辅助进程】【独立启动】按command创建无控制台辅助进程，返回输入输出管道，不保留等待线程
pub(crate) fn spawn(mut command: Command) -> io::Result<Pipes> {
    command.creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW);
    let mut child = command.spawn()?;
    Ok(Pipes {
        stdin: child.stdin.take(),
        stdout: child.stdout.take(),
    })
}
