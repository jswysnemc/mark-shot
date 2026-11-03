//! 【截图历史】【安全文件】固定目录描述符、拒绝链接与原子文件写入

use std::{
    fs::{self, File, OpenOptions},
    io,
    os::{
        fd::AsRawFd,
        unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
    time::Duration,
};

use nix::{
    errno::Errno,
    fcntl::{AtFlags, Flock, OFlag, openat},
    sys::stat::{Mode, SFlag, fstatat},
    unistd::{UnlinkatFlags, unlinkat},
};

use super::identity;

pub(super) struct Directory {
    file: File,
}

impl Directory {
    /// 【截图历史】【目录打开】按 create 决定是否创建 path；固定目录身份，拒绝末级目录链接
    pub fn open(path: &Path, create: bool) -> io::Result<Option<Self>> {
        if create {
            fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(path)?;
        }
        let file = match OpenOptions::new()
            .read(true)
            .custom_flags(nix::libc::O_DIRECTORY | nix::libc::O_NOFOLLOW)
            .open(path)
        {
            Ok(file) => file,
            Err(error) if !create && error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        Ok(Some(Self { file }))
    }

    /// 【截图历史】【描述符路径】返回已打开目录的内核路径；父路径被替换时仍访问原目录
    pub fn path(&self) -> PathBuf {
        PathBuf::from(format!("/proc/self/fd/{}", self.file.as_raw_fd()))
    }

    /// 【截图历史】【串行写锁】返回当前目录的独占锁；等待最多一秒，不接受链接或特殊文件
    pub fn lock(&self) -> io::Result<Flock<File>> {
        let file = File::from(openat(
            &self.file,
            ".history.lock",
            OFlag::O_RDWR
                | OFlag::O_CREAT
                | OFlag::O_NOFOLLOW
                | OFlag::O_CLOEXEC
                | OFlag::O_NONBLOCK,
            Mode::from_bits_truncate(0o600),
        )?);
        if !regular(&file.metadata()?) {
            return Err(io::Error::other("invalid history lock"));
        }
        crate::utils::fs_lock::acquire(file, Duration::from_secs(1))
    }

    /// 【截图历史】【拥有文件】列出合法普通历史文件及字节数；按文件名从新到旧排序，忽略链接
    pub fn owned(&self) -> io::Result<Vec<(String, u64)>> {
        let mut result = Vec::new();
        for item in fs::read_dir(self.path())? {
            let item = item?;
            let Ok(name) = item.file_name().into_string() else {
                continue;
            };
            if identity::timestamp(&name).is_none() {
                continue;
            }
            if let Ok(stat) = fstatat(&self.file, name.as_str(), AtFlags::AT_SYMLINK_NOFOLLOW)
                && SFlag::from_bits_truncate(stat.st_mode & nix::libc::S_IFMT) == SFlag::S_IFREG
                && stat.st_nlink == 1
            {
                result.push((name, stat.st_size.max(0) as u64));
            }
        }
        result.sort_unstable_by(|left, right| right.0.cmp(&left.0));
        Ok(result)
    }

    /// 【截图历史】【文件打开】只读 name 指定的合法普通历史文件；通过目录描述符拒绝路径穿越和链接
    pub fn read(&self, name: &str) -> io::Result<File> {
        if identity::timestamp(name).is_none() {
            return Err(io::Error::other("invalid history identity"));
        }
        let file = File::from(openat(
            &self.file,
            name,
            OFlag::O_RDONLY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC | OFlag::O_NONBLOCK,
            Mode::empty(),
        )?);
        if !regular(&file.metadata()?) {
            return Err(io::Error::other("invalid history file"));
        }
        Ok(file)
    }

    /// 【截图历史】【文件删除】删除合法 name；不存在视为成功，特殊文件和链接返回错误
    pub fn remove(&self, name: &str) -> io::Result<()> {
        if identity::timestamp(name).is_none() {
            return Err(io::Error::other("invalid history identity"));
        }
        match fstatat(&self.file, name, AtFlags::AT_SYMLINK_NOFOLLOW) {
            Ok(stat)
                if SFlag::from_bits_truncate(stat.st_mode & nix::libc::S_IFMT)
                    == SFlag::S_IFREG
                    && stat.st_nlink == 1 => {}
            Ok(_) => return Err(io::Error::other("invalid history file")),
            Err(Errno::ENOENT) => return Ok(()),
            Err(error) => return Err(error.into()),
        }
        match unlinkat(&self.file, name, UnlinkatFlags::NoRemoveDir) {
            Ok(()) | Err(Errno::ENOENT) => Ok(()),
            Err(error) => Err(error.into()),
        }
    }

    /// 【截图历史】【目录同步】将名称新增与删除提交到持久存储；返回同步结果
    pub fn sync(&self) -> io::Result<()> {
        self.file.sync_all()
    }
}

/// 【截图历史】【普通文件】判断 metadata 是否为单链接普通文件；避免读取或写入外部硬链接目标
fn regular(metadata: &fs::Metadata) -> bool {
    metadata.is_file() && metadata.nlink() == 1
}
