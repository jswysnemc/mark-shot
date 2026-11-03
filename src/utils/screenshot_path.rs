//! 【截图保存】【路径规划】共用默认目录与时间文件名，不提前写入图像

use std::{
    io,
    path::{Path, PathBuf},
};

/// 【截图保存】【默认目标】按 directory 覆盖或保存配置规划 PNG 路径；返回路径，不创建目录或文件
pub fn planned(directory: Option<&Path>) -> io::Result<PathBuf> {
    let now = chrono::Local::now();
    let save = &crate::config::get().save;
    let root = match directory {
        Some(directory) => directory.to_path_buf(),
        None => dirs::picture_dir()
            .or_else(|| dirs::home_dir().map(|home| home.join("Pictures")))
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "pictures and home directories are unavailable",
                )
            })?
            .join(&save.directory)
            .join(now.format(&save.month_format).to_string()),
    };
    std::path::absolute(root.join(format!("{}.png", now.format(&save.filename_format))))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【截图保存测试】【纯路径规划】显式目录不增加默认月份，也不创建尚不存在的目录
    #[test]
    fn override_directory_is_not_created_or_extended() {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("not-created");
        let target = planned(Some(&directory)).unwrap();
        assert_eq!(target.parent(), Some(directory.as_path()));
        assert_eq!(target.extension().unwrap(), "png");
        assert!(!directory.exists());
    }
}
