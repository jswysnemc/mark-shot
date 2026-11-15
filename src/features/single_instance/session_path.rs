//! 【单实例】【会话路径】使用内核合成器身份或显示名区分 Wayland 会话，目录与锁文件只属于当前用户

use nix::sys::socket::{getsockopt, sockopt::PeerCredentials};
use std::{
    ffi::OsStr,
    fs::{self, DirBuilder, File, OpenOptions},
    os::unix::{
        ffi::OsStrExt,
        fs::{DirBuilderExt, FileTypeExt, MetadataExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub(super) struct SessionPath {
    pub socket: PathBuf,
    lock: PathBuf,
    pub uid: u32,
}

impl SessionPath {
    /// 【单实例】【内核会话】按 connection 的对端身份与显示名创建当前运行目录内的路径，返回校验错误，不创建表面
    pub fn for_connection(connection: &crate::backend::session::Session) -> crate::AppResult<Self> {
        let connection = match connection {
            crate::backend::session::Session::Wayland(connection) => connection,
            crate::backend::session::Session::X11(display) => {
                let root = std::env::var_os("XDG_RUNTIME_DIR")
                    .ok_or("single-instance capture requires XDG_RUNTIME_DIR")?;
                let uid = nix::unistd::Uid::effective().as_raw();
                let name = std::env::var("DISPLAY").unwrap_or_default();
                let identity = crc32fast::hash(name.as_bytes());
                let mut path = Self::new(Path::new(&root), 1, uid)?;
                let directory = path
                    .socket
                    .parent()
                    .ok_or("invalid X11 runtime directory")?;
                let stem = format!("x11-{identity:08x}-{}", display.screen().root);
                path.lock = directory.join(format!("{stem}.lock"));
                path.socket = directory.join(format!("{stem}.sock"));
                return Ok(path);
            }
        };
        let peer = getsockopt(&connection.backend().poll_fd(), PeerCredentials)?;
        let display = std::env::var_os("WAYLAND_DISPLAY");
        let identity = wayland_identity(peer.pid(), peer.uid(), display.as_deref())?;
        let root = std::env::var_os("XDG_RUNTIME_DIR")
            .ok_or("single-instance capture requires XDG_RUNTIME_DIR")?;
        Self::new(Path::new(&root), identity, peer.uid())
    }

    /// 【单实例】【私有路径】按 root、compositor_identity 与 uid 创建私有目录，返回短套接字及锁路径，拒绝符号链接和其他用户目录
    pub fn new(root: &Path, compositor_identity: i32, uid: u32) -> crate::AppResult<Self> {
        private_directory(root, uid)?;
        if compositor_identity <= 0 {
            return Err("single-instance compositor identity is invalid".into());
        }
        let directory = root.join("mark-shot-instance");
        match DirBuilder::new().mode(0o700).create(&directory) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.into()),
        }
        private_directory(&directory, uid)?;
        let socket = directory.join(format!("capture-{compositor_identity}.sock"));
        if socket.as_os_str().as_bytes().len() >= 104 {
            return Err("single-instance socket path exceeds the Unix socket limit".into());
        }
        Ok(Self {
            socket,
            lock: directory.join(format!("capture-{compositor_identity}.lock")),
            uid,
        })
    }

    /// 【单实例】【占用文件】无额外参数，打开私有普通锁文件；返回独立句柄，拒绝链接、特殊文件及其他用户文件
    pub fn lock_file(&self) -> crate::AppResult<File> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_CLOEXEC | nix::libc::O_NONBLOCK)
            .open(&self.lock)?;
        let metadata = file.metadata()?;
        if !metadata.is_file() || metadata.uid() != self.uid || metadata.nlink() != 1 {
            return Err("single-instance lock is not a private ordinary file".into());
        }
        Ok(file)
    }

    /// 【单实例】【旧路径检查】无额外参数，返回当前私有套接字的设备与 inode；路径缺失返回空值，非套接字返回错误
    pub fn socket_identity(&self) -> crate::AppResult<Option<(u64, u64)>> {
        match fs::symlink_metadata(&self.socket) {
            Ok(metadata) if metadata.file_type().is_socket() && metadata.uid() == self.uid => {
                Ok(Some((metadata.dev(), metadata.ino())))
            }
            Ok(_) => Err("single-instance path is not a private socket".into()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    /// 【单实例】【路径释放】仅删除 identity 仍对应的本次套接字，无返回值，避免退出时删除其他进程新建的路径
    pub fn remove_socket(&self, identity: (u64, u64)) {
        if self.socket_identity().ok().flatten() == Some(identity) {
            let _ = fs::remove_file(&self.socket);
        }
    }
}

/// 【单实例】【Wayland身份】按 compositor_pid、uid 与 display 返回正数会话标识，PID 不可见时使用显示名，缺失名称按 wayland-0 处理
pub(super) fn wayland_identity(
    compositor_pid: i32,
    uid: u32,
    display: Option<&OsStr>,
) -> crate::AppResult<i32> {
    // 1. 【单实例】【内核身份】始终核对用户，可见合成器保持原有 PID 标识
    if compositor_pid < 0 || uid != nix::unistd::Uid::effective().as_raw() {
        return Err("Wayland compositor does not belong to the current user".into());
    }
    if compositor_pid > 0 {
        return Ok(compositor_pid);
    }
    // 2. 【单实例】【命名空间回退】对端在其他 PID 命名空间时，按显示名生成稳定正数标识
    let display = display.unwrap_or_else(|| OsStr::new("wayland-0"));
    Ok((crc32fast::hash(display.as_bytes()) & 0x7fff_ffff).max(1) as i32)
}

/// 【单实例】【目录认证】按 path 与 uid 检查真实私有目录，返回权限或文件类型错误，不修改已有目录权限
fn private_directory(path: &Path, uid: u32) -> crate::AppResult {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir() || metadata.uid() != uid || metadata.mode() & 0o077 != 0 {
        return Err(
            "single-instance runtime directory must be private and owned by this user".into(),
        );
    }
    Ok(())
}
