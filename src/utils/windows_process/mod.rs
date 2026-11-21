//! 【Windows内部进程】【静默启动】内部自身进程禁用系统启动反馈，保留标准流、环境覆盖及会话回收

mod child;
mod command_line;
mod environment;
mod handles;
#[cfg(test)]
mod tests;

pub(crate) use child::Child;
use std::{
    fs::File,
    io,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    process::Command,
};
use windows_sys::Win32::System::Threading::{
    CREATE_UNICODE_ENVIRONMENT, CreateProcessW, EXTENDED_STARTUPINFO_PRESENT, PROCESS_INFORMATION,
    STARTF_FORCEOFFFEEDBACK, STARTF_USESTDHANDLES, STARTUPINFOEXW,
};

pub(crate) enum Stream {
    Null,
    Piped,
    File(File),
}

pub(crate) struct Streams {
    pub stdin: Stream,
    pub stdout: Stream,
    pub stderr: Option<File>,
}

/// 【Windows内部进程】【静默创建】按 command 的路径、参数、当前目录和显式环境覆盖启动继承环境的自身进程
/// flags 指定进程创建标志，streams 指定实际标准流；返回进程及父端管道，不读取Command中的标准流设置或env_clear状态
pub(crate) fn spawn(command: &Command, flags: u32, streams: Streams) -> io::Result<Child> {
    if !std::path::Path::new(command.get_program()).is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "internal executable must be absolute",
        ));
    }
    let program = command_line::wide(command.get_program())?;
    let mut arguments = command_line::encode(command)?;
    let environment = environment::build(command)?;
    let directory = command
        .get_current_dir()
        .map(|path| command_line::wide(path.as_os_str()))
        .transpose()?;
    let pipes = handles::Prepared::new(streams)?;
    let inherited = [
        pipes.input.as_raw_handle(),
        pipes.output.as_raw_handle(),
        pipes.error.as_raw_handle(),
    ];
    let mut attributes = handles::Attributes::new(&inherited)?;
    let mut startup: STARTUPINFOEXW = unsafe { std::mem::zeroed() };
    startup.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXW>() as u32;
    // 1. 【Windows内部进程】【启动反馈】反馈标志属于STARTUPINFO，不能与CREATE_NO_WINDOW等创建标志混用
    startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES | STARTF_FORCEOFFFEEDBACK;
    startup.StartupInfo.hStdInput = inherited[0];
    startup.StartupInfo.hStdOutput = inherited[1];
    startup.StartupInfo.hStdError = inherited[2];
    startup.lpAttributeList = attributes.as_mut_ptr();
    let mut process: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
    // 2. 【Windows内部进程】【句柄白名单】只继承本次标准流，文件锁与其他截图管道不会交给子进程
    if unsafe {
        CreateProcessW(
            program.as_ptr(),
            arguments.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1,
            flags | CREATE_UNICODE_ENVIRONMENT | EXTENDED_STARTUPINFO_PRESENT,
            environment.as_ptr().cast(),
            directory
                .as_ref()
                .map_or(std::ptr::null(), |path| path.as_ptr()),
            &startup.StartupInfo,
            &mut process,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let process_handle = unsafe { OwnedHandle::from_raw_handle(process.hProcess) };
    let _thread_handle = unsafe { OwnedHandle::from_raw_handle(process.hThread) };
    Ok(Child {
        process: process_handle,
        id: process.dwProcessId,
        stdin: pipes.stdin,
        stdout: pipes.stdout,
    })
}
