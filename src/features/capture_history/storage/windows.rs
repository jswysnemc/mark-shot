//! 【Windows截图历史】【固定目录】目录句柄拒绝重解析且禁止删除共享，历史文件只接受合法单链接普通文件

use super::identity;
use std::{
    fs::{self, File, OpenOptions},
    io,
    os::windows::{ffi::OsStringExt, fs::OpenOptionsExt, io::AsRawHandle},
    path::{Path, PathBuf},
    time::Duration,
};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ, FILE_SHARE_WRITE,
    GetFinalPathNameByHandleW,
};
pub(super) struct Directory {
    file: File,
}
impl Directory {
    /// 【Windows截图历史】【目录打开】按create创建path并固定原生目录身份，返回可选范围，拒绝末级重解析点
    pub fn open(path: &Path, create: bool) -> io::Result<Option<Self>> {
        if create {
            fs::create_dir_all(path)?;
        }
        let file = match OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(path)
        {
            Ok(file) => file,
            Err(error) if !create && error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        use std::os::windows::fs::MetadataExt;
        if !file.metadata()?.is_dir() || file.metadata()?.file_attributes() & 0x400 != 0 {
            return Err(io::Error::other("invalid history directory"));
        }
        Ok(Some(Self { file }))
    }
    /// 【Windows截图历史】【句柄路径】无参数，返回系统确认的当前目录路径，目录存活期间禁止重命名或删除
    pub fn path(&self) -> PathBuf {
        let mut buffer = vec![0_u16; 32768];
        let count = unsafe {
            GetFinalPathNameByHandleW(
                self.file.as_raw_handle(),
                buffer.as_mut_ptr(),
                buffer.len() as u32,
                0,
            )
        } as usize;
        if count == 0 || count >= buffer.len() {
            return PathBuf::new();
        }
        PathBuf::from(std::ffi::OsString::from_wide(&buffer[..count]))
    }
    /// 【Windows截图历史】【串行锁】无参数，返回固定目录内普通文件锁，等待最多一秒，不接受重解析目标
    pub fn lock(&self) -> io::Result<crate::utils::fs_lock::Lock> {
        let file = crate::utils::platform::open_regular(&self.path().join(".history.lock"), true)?;
        if !crate::utils::platform::regular(&file)? {
            return Err(io::Error::other("invalid history lock"));
        }
        crate::utils::fs_lock::acquire(file, Duration::from_secs(1))
    }
    /// 【Windows截图历史】【拥有列表】无参数，返回合法历史普通文件与长度，按名称从新到旧排序
    pub fn owned(&self) -> io::Result<Vec<(String, u64)>> {
        let mut result = Vec::new();
        for entry in fs::read_dir(self.path())? {
            let entry = entry?;
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if identity::timestamp(&name).is_none() {
                continue;
            }
            if let Ok(file) = self.read(&name) {
                result.push((name, file.metadata()?.len()));
            }
        }
        result.sort_unstable_by(|left, right| right.0.cmp(&left.0));
        Ok(result)
    }
    /// 【Windows截图历史】【原图打开】按name返回普通单链接只读文件，拒绝路径穿越与重解析目标
    pub fn read(&self, name: &str) -> io::Result<File> {
        if identity::timestamp(name).is_none() {
            return Err(io::Error::other("invalid history identity"));
        }
        let file = crate::utils::platform::open_regular(&self.path().join(name), false)?;
        if !crate::utils::platform::regular(&file)? {
            return Err(io::Error::other("invalid history file"));
        }
        Ok(file)
    }
    /// 【Windows截图历史】【文件删除】按name确认普通文件后删除，返回结果，缺失视为成功
    pub fn remove(&self, name: &str) -> io::Result<()> {
        match self.read(name) {
            Ok(file) => drop(file),
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        }
        fs::remove_file(self.path().join(name))
    }
    /// 【Windows截图历史】【目录同步】无参数，文件与原子替换已完成系统交接，Windows目录句柄不执行Unix目录同步，返回成功状态
    pub fn sync(&self) -> io::Result<()> {
        Ok(())
    }
}
