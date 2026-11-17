//! 【截图另存为】【安全落盘】拒绝符号链接并优先采用同目录原子写入，兼容文档门户 FUSE 直写

use std::{
    fs,
    io::{self, Write},
    path::Path,
};

/// 【截图另存为】【目标校验】验证已存在的保存目标为普通单链接文件，拒绝符号链接或目录覆盖
#[cfg(target_os = "linux")]
fn validate_existing_target(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::MetadataExt;
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err),
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.nlink() != 1 {
        return Err(io::Error::other("target is not a single-link regular file"));
    }
    Ok(())
}

/// 【截图另存为】【Windows目标校验】检查已存在目标为普通单链接文件，拒绝重解析点与目录
#[cfg(target_os = "windows")]
fn validate_existing_target(path: &Path) -> io::Result<()> {
    let file = match crate::utils::platform::open_regular(path, false) {
        Ok(file) => file,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err),
    };
    if !crate::utils::platform::regular(&file)? {
        return Err(io::Error::other("target is not a single-link regular file"));
    }
    Ok(())
}

/// 【截图另存为】【门户直写回退】在同目录临时文件受沙箱门户限制时，使用不跟随符号链接方式截断直写
fn write_direct_regular(png: &[u8], path: &Path) -> io::Result<()> {
    validate_existing_target(path)?;
    #[cfg(target_os = "linux")]
    let mut file = {
        use std::os::unix::fs::OpenOptionsExt;
        fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .custom_flags(libc_o_nofollow())
            .open(path)?
    };
    #[cfg(target_os = "windows")]
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    file.write_all(png)?;
    file.sync_all()
}

/// 【截图另存为】【O_NOFOLLOW常量】返回 Linux 平台对应的 O_NOFOLLOW 标志位以阻断符号链接跟随
#[cfg(target_os = "linux")]
const fn libc_o_nofollow() -> i32 {
    nix::libc::O_NOFOLLOW
}

/// 【截图另存为】【PNG落盘】把冻结的 png 字节安全写入门户授权的 path，失败返回具体 IO 错误
pub fn write_png(png: &[u8], path: &Path) -> io::Result<()> {
    // 1. 先拦截符号链接与非法目录目标，防止覆盖外部敏感文件
    validate_existing_target(path)?;
    let parent = path
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .unwrap_or(Path::new("."));

    // 2. 优先在父目录创建临时文件并原子替换，若处于受限文档门户 FUSE 目录则回退安全直写
    match tempfile::NamedTempFile::new_in(parent) {
        Ok(mut temporary) => {
            temporary.write_all(png)?;
            temporary.as_file().sync_all()?;
            validate_existing_target(path)?;
            temporary.persist(path).map_err(|err| err.error)?;
            #[cfg(target_os = "linux")]
            if let Ok(dir) = fs::File::open(parent) {
                let _ = dir.sync_all();
            }
            Ok(())
        }
        Err(err)
            if matches!(
                err.kind(),
                io::ErrorKind::PermissionDenied | io::ErrorKind::Unsupported
            ) || super::portal::is_document_portal_path(path) =>
        {
            write_direct_regular(png, path)
        }
        Err(err) => Err(err),
    }
}
