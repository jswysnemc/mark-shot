//! 【Windows主题】【目录事件】系统目录通知在原子替换后仍有效，不创建轮询线程

use std::{io, os::windows::ffi::OsStrExt, path::Path};
use windows_sys::Win32::{
    Foundation::{HANDLE, INVALID_HANDLE_VALUE, WAIT_OBJECT_0},
    Storage::FileSystem::{
        FILE_NOTIFY_CHANGE_FILE_NAME, FILE_NOTIFY_CHANGE_LAST_WRITE, FILE_NOTIFY_CHANGE_SIZE,
        FindCloseChangeNotification, FindFirstChangeNotificationW, FindNextChangeNotification,
    },
    System::Threading::WaitForSingleObject,
};

pub(super) struct Source(HANDLE);
impl Source {
    /// 【Windows主题】【目录登记】按path监听父目录，返回原生来源，缺失目录先创建，不打开配置文件
    pub(super) fn new(path: &Path) -> io::Result<Self> {
        let parent = path
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        std::fs::create_dir_all(parent)?;
        let directory = parent
            .as_os_str()
            .encode_wide()
            .chain([0])
            .collect::<Vec<_>>();
        let handle = unsafe {
            FindFirstChangeNotificationW(
                directory.as_ptr(),
                0,
                FILE_NOTIFY_CHANGE_FILE_NAME
                    | FILE_NOTIFY_CHANGE_LAST_WRITE
                    | FILE_NOTIFY_CHANGE_SIZE,
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            Err(io::Error::last_os_error())
        } else {
            Ok(Self(handle))
        }
    }
    /// 【Windows主题】【事件消费】无参数，消费已到达目录变化并重新登记，返回是否需要有界读取主题
    pub(super) fn drain(&self) -> io::Result<bool> {
        if unsafe { WaitForSingleObject(self.0, 0) } != WAIT_OBJECT_0 {
            return Ok(false);
        }
        if unsafe { FindNextChangeNotification(self.0) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(true)
    }
    /// 【Windows主题】【共同等待】无参数，返回系统目录来源值，调用方不得关闭
    pub(super) fn handle(&self) -> isize {
        self.0 as isize
    }
}
impl Drop for Source {
    /// 【Windows主题】【来源关闭】无参数，关闭本次目录通知，不影响其他进程监听，无返回值
    fn drop(&mut self) {
        unsafe {
            FindCloseChangeNotification(self.0);
        }
    }
}
