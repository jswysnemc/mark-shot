//! 【截图窗口】【跨进程协调】短期租约通知自身窗口隐藏，持有者退出后自动恢复

mod lease;
mod watch;
pub use lease::Guard;
pub use watch::Watch;

use std::{
    fs::{self, File, OpenOptions},
    hash::{Hash, Hasher},
    io,
    path::{Path, PathBuf},
};

/// 【截图窗口】【会话目录】无参数，返回当前用户与显示会话独占的运行目录，不包含图像或配置正文
fn directory() -> io::Result<PathBuf> {
    let base = dirs::runtime_dir()
        .or_else(dirs::cache_dir)
        .ok_or_else(|| io::Error::other("无法取得截图窗口协调目录"))?;
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    // 1. 【截图窗口】【显示隔离】Wayland 会话忽略不稳定的 XWayland DISPLAY，直接启动与桌面启动必须进入同一目录
    let display = std::env::var_os("WAYLAND_DISPLAY")
        .filter(|value| !value.is_empty())
        .or_else(|| std::env::var_os("DISPLAY"))
        .or_else(|| std::env::var_os("SESSIONNAME"));
    display.hash(&mut hash);
    let path = base
        .join("mark-shot/capture-visibility")
        .join(format!("{:x}", hash.finish()));
    fs::create_dir_all(&path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(path)
}

/// 【截图窗口】【活跃记录】按 path 返回文件锁是否仍由进程持有，已退出进程留下的文件不构成隐藏请求
fn active(path: &Path) -> bool {
    let Ok(file) = OpenOptions::new().read(true).write(true).open(path) else {
        return false;
    };
    matches!(file.try_lock(), Err(std::fs::TryLockError::WouldBlock))
}

/// 【截图窗口】【记录枚举】按 directory 和 prefix 返回仍持有文件锁的记录路径，读取失败时返回错误
fn records(directory: &Path, prefix: &str) -> io::Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        if entry.file_name().to_string_lossy().starts_with(prefix) && active(&entry.path()) {
            paths.push(entry.path());
        }
    }
    paths.sort();
    Ok(paths)
}

/// 【截图窗口】【锁定记录】按 directory 和 prefix 创建生命周期受文件锁保护的临时记录，返回持有者
fn record(directory: &Path, prefix: &str) -> io::Result<tempfile::NamedTempFile> {
    let file = tempfile::Builder::new()
        .prefix(prefix)
        .tempfile_in(directory)?;
    File::lock(file.as_file())?;
    Ok(file)
}

#[cfg(test)]
mod tests;
