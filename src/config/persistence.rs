//! 【应用配置】【原子更新】统一历史和翻译的进程锁，单项保存保留其他窗口刚写入的字段

use std::{
    fs::{self},
    io::{self, Read, Write},
    path::Path,
    time::Duration,
};

/// 【应用配置】【安全读取】按 path 返回普通私有配置内容，拒绝链接、多重硬链接、非普通文件和超过一 MiB 的内容
pub(crate) fn read(path: &Path) -> io::Result<String> {
    let file = crate::utils::platform::open_regular(path, false)?;
    if !crate::utils::platform::regular(&file)? || file.metadata()?.len() > 1024 * 1024 {
        return Err(io::Error::other("invalid config file"));
    }
    let mut text = String::new();
    file.take(1024 * 1024 + 1).read_to_string(&mut text)?;
    if text.len() > 1024 * 1024 {
        return Err(io::Error::other("config file is too large"));
    }
    Ok(text)
}

/// 【应用配置】【串行保存】在 path 的共享锁内执行 edit，返回业务结果；edit 提供更新后的文本，失败保持原文件不变
pub(crate) fn update<T>(
    path: &Path,
    edit: impl FnOnce(&str) -> io::Result<(String, T)>,
) -> io::Result<T> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    // 1. 【应用配置】【兼容锁名】保留已有历史窗口使用的锁名称，两个业务不能分别加锁后覆盖彼此的写入
    let lock =
        crate::utils::platform::open_regular(&path.with_extension("toml.history.lock"), true)?;
    if !crate::utils::platform::regular(&lock)? {
        return Err(io::Error::other("invalid config lock"));
    }
    let _lock = crate::utils::fs_lock::acquire(lock, Duration::from_secs(1))?;
    let previous = match read(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            if super::document::is_json(path, "") {
                "{}".to_owned()
            } else {
                super::INITIAL.to_owned()
            }
        }
        Err(error) => return Err(error),
    };
    let (text, result) = edit(&previous)?;
    if text.len() > 1024 * 1024 {
        return Err(io::Error::other("updated config file is too large"));
    }
    // 2. 【应用配置】【原子替换】私有临时文件先同步再替换，原配置不会出现部分写入
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(text.as_bytes())?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|error| error.error)?;
    #[cfg(target_os = "linux")]
    fs::File::open(parent)?.sync_all()?;
    Ok(result)
}
