//! 【工具偏好】【平台存储】选择跨平台状态路径，并通过私有临时文件原子替换偏好

use std::{
    io,
    io::Write,
    path::{Path, PathBuf},
};

/// 【工具偏好】【目录选择】按 state_directory 和 local_directory 返回偏好路径，目录均不可用时返回空值
pub(super) fn select(
    state_directory: Option<PathBuf>,
    local_directory: Option<PathBuf>,
) -> Option<PathBuf> {
    state_directory
        .or(local_directory)
        .map(|directory| directory.join("mark-shot").join("tools.toml"))
}

/// 【工具偏好】【系统目录】无参数，返回当前平台的偏好路径，不创建文件
pub(super) fn path() -> Option<PathBuf> {
    select(dirs::state_dir(), dirs::data_local_dir())
}

/// 【工具偏好】【原子写入】将 file 编码后保存到 path，返回写入结果，失败不替换原文件
pub(super) fn write(path: &Path, file: &super::File) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("tool preferences directory is unavailable"))?;
    let text = toml::to_string(file).map_err(io::Error::other)?;
    // 1. 【工具偏好】【私有暂存】在目标目录创建独立临时文件，同步内容后才替换旧偏好
    std::fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(text.as_bytes())?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|error| error.error)?;
    Ok(())
}
