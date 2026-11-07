//! 【Linux主题】【目录监听】监听目标文件的完成写入、原子替换与删除，避免文件替换使旧监听失效

use nix::libc;
use std::{
    ffi::{CString, OsString},
    fs::File,
    io::{self, Read},
    os::{
        fd::{AsFd, BorrowedFd, FromRawFd},
        unix::ffi::{OsStrExt, OsStringExt},
    },
    path::Path,
};

pub(super) struct Source {
    file: File,
    name: OsString,
}

impl Source {
    /// 【Linux主题】【目录登记】按 path 创建非阻塞来源，返回来源或系统错误；创建缺失父目录但不写入配置
    pub(super) fn new(path: &Path) -> io::Result<Self> {
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let name = path
            .file_name()
            .ok_or_else(|| io::Error::other("theme config requires a file name"))?
            .to_owned();
        std::fs::create_dir_all(parent)?;
        let directory = CString::new(parent.as_os_str().as_bytes()).map_err(io::Error::other)?;
        // 1. 【Linux主题】【独立来源】内核返回的新描述符只由本来源关闭，非阻塞读取不会占用界面线程
        let descriptor = unsafe { libc::inotify_init1(libc::IN_NONBLOCK | libc::IN_CLOEXEC) };
        if descriptor < 0 {
            return Err(io::Error::last_os_error());
        }
        let file = unsafe { File::from_raw_fd(descriptor) };
        let mask = libc::IN_CLOSE_WRITE
            | libc::IN_MOVED_TO
            | libc::IN_DELETE
            | libc::IN_MOVE_SELF
            | libc::IN_DELETE_SELF;
        let result = unsafe { libc::inotify_add_watch(descriptor, directory.as_ptr(), mask) };
        if result < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self { file, name })
    }

    /// 【Linux主题】【批量消费】无参数，读取已到达事件并返回目标文件是否变化，无事件时立即返回，不保留周期期限
    pub(super) fn drain(&self) -> io::Result<bool> {
        let mut changed = false;
        let mut buffer = [0_u8; 4096];
        loop {
            let count = match (&self.file).read(&mut buffer) {
                Ok(0) => return Err(io::Error::other("theme watch closed")),
                Ok(count) => count,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(changed),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            };
            let mut offset = 0;
            let header = size_of::<libc::inotify_event>();
            while offset + header <= count {
                // 2. 【Linux主题】【长度校验】字节缓冲不保证结构对齐，先读取完整头，再校验内核提供的名称长度
                let event = unsafe {
                    std::ptr::read_unaligned(
                        buffer.as_ptr().add(offset).cast::<libc::inotify_event>(),
                    )
                };
                let end = offset + header + event.len as usize;
                if end > count {
                    return Err(io::Error::other("incomplete theme notification"));
                }
                let bytes = &buffer[offset + header..end];
                let length = bytes
                    .iter()
                    .position(|byte| *byte == 0)
                    .unwrap_or(bytes.len());
                changed |= event.mask & libc::IN_Q_OVERFLOW != 0
                    || OsString::from_vec(bytes[..length].to_vec()) == self.name;
                if event.mask & (libc::IN_IGNORED | libc::IN_MOVE_SELF | libc::IN_DELETE_SELF) != 0
                {
                    return Err(io::Error::other(
                        "theme configuration directory was removed or moved",
                    ));
                }
                offset = end;
            }
        }
    }
}

impl AsFd for Source {
    /// 【Linux主题】【等待接口】返回内核通知的借用描述符，无参数，不转移关闭责任
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.file.as_fd()
    }
}
