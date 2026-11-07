//! 【外部任务】【所属进程】Linux进程组与Windows作业对象统一拥有本次命令及后代，取消后完整回收

use std::io;

#[cfg(target_os = "linux")]
pub(crate) struct Group(u32);
#[cfg(target_os = "windows")]
pub(crate) struct Group(isize);

impl Group {
    /// 【外部任务】【所属登记】按child创建本次任务守卫，返回进程身份或系统错误，不引用其他进程
    pub(crate) fn new(child: &tokio::process::Child) -> io::Result<Self> {
        #[cfg(target_os = "linux")]
        {
            child
                .id()
                .map(Self)
                .ok_or_else(|| io::Error::other("external command has no process identity"))
        }
        #[cfg(target_os = "windows")]
        {
            use windows_sys::Win32::{
                Foundation::CloseHandle,
                System::JobObjects::{
                    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
                    SetInformationJobObject,
                },
            };
            let process = child
                .raw_handle()
                .ok_or_else(|| io::Error::other("external command has no process handle"))?;
            let job = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
            if job.is_null() {
                return Err(io::Error::last_os_error());
            }
            let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if unsafe {
                SetInformationJobObject(
                    job,
                    JobObjectExtendedLimitInformation,
                    (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                    size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                )
            } == 0
                || unsafe { AssignProcessToJobObject(job, process) } == 0
            {
                let error = io::Error::last_os_error();
                unsafe {
                    CloseHandle(job);
                }
                return Err(error);
            }
            Ok(Self(job as isize))
        }
    }
    /// 【外部任务】【任务终止】无参数，终止本次所属组并解除身份，不保留可复用PID或系统句柄，无返回值
    pub(crate) fn terminate(&mut self) {
        let owned = std::mem::take(&mut self.0);
        #[cfg(target_os = "linux")]
        if owned > 1 {
            let _ = nix::sys::signal::kill(
                nix::unistd::Pid::from_raw(-(owned as i32)),
                nix::sys::signal::Signal::SIGKILL,
            );
        }
        #[cfg(target_os = "windows")]
        if owned != 0 {
            unsafe {
                windows_sys::Win32::Foundation::CloseHandle(owned as _);
            }
        }
    }
}
impl Drop for Group {
    /// 【外部任务】【异常回收】无参数，关闭尚未解除的任务范围，无返回值，系统同时结束所属后代
    fn drop(&mut self) {
        self.terminate();
    }
}

/// 【外部任务】【组隔离】按command设置新进程组或无控制台标志，无返回值，不修改命令正文和环境
pub(crate) fn configure(command: &mut tokio::process::Command) {
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::process::CommandExt;
        command.as_std_mut().process_group(0);
    }
    #[cfg(target_os = "windows")]
    {
        command.creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW);
    }
}
