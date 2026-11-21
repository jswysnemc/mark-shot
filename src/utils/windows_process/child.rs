//! 【Windows内部进程】【会话回收】进程句柄与父端管道分别持有，关闭句柄不终止独立辅助窗口

use std::{
    fs::File,
    io,
    os::windows::io::{AsRawHandle, OwnedHandle},
    process::ExitStatus,
};
use windows_sys::Win32::{
    Foundation::{WAIT_OBJECT_0, WAIT_TIMEOUT},
    System::Threading::{GetExitCodeProcess, INFINITE, TerminateProcess, WaitForSingleObject},
};

pub(crate) struct Child {
    pub(super) process: OwnedHandle,
    pub(super) id: u32,
    pub stdin: Option<File>,
    pub stdout: Option<File>,
}

impl Child {
    /// 【Windows内部进程】【进程编号】无参数，返回创建时取得的进程编号，用于前台权限交接
    pub fn id(&self) -> u32 {
        self.id
    }

    /// 【Windows内部进程】【退出状态】无参数，读取已结束进程的状态，返回退出码或系统错误
    fn status(&self) -> io::Result<ExitStatus> {
        use std::os::windows::process::ExitStatusExt;
        let mut code = 0;
        if unsafe { GetExitCodeProcess(self.process.as_raw_handle(), &mut code) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(ExitStatus::from_raw(code))
    }

    /// 【Windows内部进程】【非阻塞领取】无参数，返回已结束进程的状态，仍运行时返回空值
    pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        match unsafe { WaitForSingleObject(self.process.as_raw_handle(), 0) } {
            WAIT_OBJECT_0 => self.status().map(Some),
            WAIT_TIMEOUT => Ok(None),
            _ => Err(io::Error::last_os_error()),
        }
    }

    /// 【Windows内部进程】【等待回收】无参数，关闭仍持有的输入端并等待结束，返回真实退出状态
    pub fn wait(&mut self) -> io::Result<ExitStatus> {
        self.stdin.take();
        if unsafe { WaitForSingleObject(self.process.as_raw_handle(), INFINITE) } != WAIT_OBJECT_0 {
            return Err(io::Error::last_os_error());
        }
        self.status()
    }

    /// 【Windows内部进程】【异常终止】无参数，终止仍运行的本次进程，返回系统结果；已结束时返回成功
    pub fn kill(&mut self) -> io::Result<()> {
        if self.try_wait()?.is_some() {
            return Ok(());
        }
        if unsafe { TerminateProcess(self.process.as_raw_handle(), 1) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}
