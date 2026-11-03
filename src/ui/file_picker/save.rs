//! 【文件选择】【安全保存】同目录原子保存和明确覆盖授权

use std::{
    fs::{self, File},
    io::{self, Write},
    os::unix::fs::MetadataExt,
    path::Path,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExistingFile {
    dev: u64,
    ino: u64,
    size: u64,
    modified: (i64, i64),
}

/// 【文件选择】【目标检查】检查 path 是否为普通文件；返回可用于覆盖确认的身份，拒绝链接与目录
pub fn existing(path: &Path) -> io::Result<Option<ExistingFile>> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    if !metadata.is_file() || metadata.nlink() != 1 {
        return Err(io::Error::other("target is not a single-link regular file"));
    }
    Ok(Some(ExistingFile {
        dev: metadata.dev(),
        ino: metadata.ino(),
        size: metadata.len(),
        modified: (metadata.mtime(), metadata.mtime_nsec()),
    }))
}

/// 【文件选择】【原子写入】把 png 写入 path，expected 是用户确认的旧文件身份；未授权覆盖或目标变化返回错误
pub fn write(png: &[u8], path: &Path, expected: Option<ExistingFile>) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(png)?;
    temporary.as_file().sync_all()?;
    // 1. 【文件选择】【覆盖核对】确认之后发生变化的目标必须重新选择，不能静默覆盖
    if existing(path)? != expected {
        return Err(io::Error::other("target changed; select the file again"));
    }
    if expected.is_some() {
        temporary.persist(path).map_err(|error| error.error)?;
    } else {
        temporary
            .persist_noclobber(path)
            .map_err(|error| error.error)?;
    }
    File::open(parent)?.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;
    /// 【文件选择测试】【覆盖授权】未确认或目标变化时保留原文件，确认有效时原子保存
    #[test]
    fn overwrite_requires_the_exact_confirmed_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("image.png");
        write(b"first", &path, None).unwrap();
        assert!(write(b"second", &path, None).is_err());
        let before = existing(&path).unwrap();
        fs::write(&path, b"changed image").unwrap();
        assert!(write(b"third", &path, before).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"changed image");
        write(b"final", &path, existing(&path).unwrap()).unwrap();
        assert_eq!(fs::read(path).unwrap(), b"final");
    }
    /// 【文件选择测试】【链接保护】符号链接不能作为保存目标，其外部目标不改变
    #[test]
    fn linked_targets_are_not_saved() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("target");
        let link = directory.path().join("image.png");
        fs::write(&target, b"private").unwrap();
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert!(existing(&link).is_err());
        assert!(write(b"changed", &link, None).is_err());
        assert_eq!(fs::read(target).unwrap(), b"private");
    }
}
