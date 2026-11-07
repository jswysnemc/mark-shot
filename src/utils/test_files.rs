//! 【平台测试】【文件链接】用各平台原生文件和目录链接构造相同的存储测试场景

use std::{io, path::Path};

/// 【平台测试】【文件链接】按target和link创建文件符号链接，返回原生结果，不修改目标内容
pub(crate) fn link_file(target: impl AsRef<Path>, link: impl AsRef<Path>) -> io::Result<()> {
    #[cfg(target_os = "linux")]
    {
        std::os::unix::fs::symlink(target, link)
    }
    #[cfg(target_os = "windows")]
    {
        std::os::windows::fs::symlink_file(target, link)
    }
}

/// 【平台测试】【目录链接】按target和link创建目录符号链接，返回原生结果，链接不获取目标所有权
pub(crate) fn link_directory(target: impl AsRef<Path>, link: impl AsRef<Path>) -> io::Result<()> {
    #[cfg(target_os = "linux")]
    {
        std::os::unix::fs::symlink(target, link)
    }
    #[cfg(target_os = "windows")]
    {
        std::os::windows::fs::symlink_dir(target, link)
    }
}
