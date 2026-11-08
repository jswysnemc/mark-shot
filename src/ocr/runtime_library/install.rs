//! 【识别运行库】【资源安装】只提取清单文件，全部验证后原子发布版本目录

use super::manifest::Distribution;
use std::{
    fs::{self, File},
    io::{self, Read},
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

/// 【识别运行库】【目录校验】验证 dir 中的 dist 运行库，返回完整性错误，不执行库代码
pub fn verify(dir: &Path, dist: Distribution) -> Result<(), String> {
    for asset in [&dist.library, &dist.shared] {
        crate::ocr::model_integrity::verify_asset(&dir.join(asset.file), asset)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

/// 【识别运行库】【文件复制】从 reader 复制允许的 name 到 dir；size 限制单文件大小，cancel 中断安装，返回错误
fn copy(
    reader: &mut impl Read,
    name: &str,
    size: u64,
    dir: &Path,
    dist: Distribution,
    cancel: &AtomicBool,
) -> Result<(), String> {
    let limit = if name == dist.library.file {
        dist.library.size
    } else if name == dist.shared.file {
        dist.shared.size
    } else if matches!(name, "LICENSE" | "ThirdPartyNotices.txt") {
        2 * 1024 * 1024
    } else {
        return Ok(());
    };
    if cancel.load(Ordering::Relaxed) {
        return Err("下载已取消 / Download cancelled".into());
    }
    if size > limit {
        return Err("运行库文件大小异常 / Invalid runtime file size".into());
    }
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dir.join(name))
        .map_err(|error| error.to_string())?;
    let copied =
        io::copy(&mut reader.take(limit + 1), &mut output).map_err(|error| error.to_string())?;
    if copied != size {
        return Err("运行库文件不完整 / Incomplete runtime file".into());
    }
    output.sync_all().map_err(|error| error.to_string())
}

/// 【识别运行库】【归档解包】将 archive 的清单文件提取到空 dir，dist 指定允许内容，cancel 控制取消，返回验证结果
pub fn extract(
    archive: &Path,
    dir: &Path,
    dist: Distribution,
    cancel: &AtomicBool,
) -> Result<(), String> {
    let file = File::open(archive).map_err(|error| error.to_string())?;
    if dist.archive.file.ends_with(".zip") {
        let mut archive = zip::ZipArchive::new(file).map_err(|error| error.to_string())?;
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).map_err(|error| error.to_string())?;
            if !entry.is_file() || entry.is_symlink() {
                continue;
            }
            let name = entry
                .enclosed_name()
                .and_then(|path| {
                    path.file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                })
                .ok_or("运行库归档路径无效 / Invalid archive path")?;
            let size = entry.size();
            copy(&mut entry, &name, size, dir, dist, cancel)?;
        }
    } else {
        let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(file));
        for entry in archive.entries().map_err(|error| error.to_string())? {
            let mut entry = entry.map_err(|error| error.to_string())?;
            if !entry.header().entry_type().is_file() {
                continue;
            }
            let path = entry.path().map_err(|error| error.to_string())?;
            if path.is_absolute()
                || path
                    .components()
                    .any(|component| matches!(component, std::path::Component::ParentDir))
            {
                return Err("运行库归档路径无效 / Invalid archive path".into());
            }
            let name = path
                .file_name()
                .ok_or("missing archive filename")?
                .to_string_lossy()
                .into_owned();
            let size = entry.size();
            copy(&mut entry, &name, size, dir, dist, cancel)?;
        }
    }
    verify(dir, dist)
}

/// 【识别运行库】【手动下载】按 dist 下载到 root，cancel 控制停止，progress 接收百分比，返回已验证目录；只能由明确下载动作调用
pub fn download(
    root: &Path,
    dist: Distribution,
    cancel: &AtomicBool,
    progress: impl Fn(u8),
) -> Result<(), String> {
    fs::create_dir_all(root).map_err(|error| error.to_string())?;
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(root.join("install.lock"))
        .map_err(|error| error.to_string())?;
    let _lock = crate::utils::fs_lock::acquire(lock, std::time::Duration::ZERO).map_err(|_| {
        "另一窗口正在安装运行库 / Runtime is being installed in another window".to_string()
    })?;
    let installed = root.join("installed");
    if verify(&installed, dist).is_ok() {
        return Ok(());
    }
    let stopped = AtomicBool::new(false);
    crate::ocr::download::fetch(
        &crate::ocr::download::agent(),
        &format!("{}/{}", super::manifest::BASE_URL, dist.archive.file),
        &dist.archive,
        root,
        cancel,
        &stopped,
        |bytes| progress((bytes * 95 / dist.archive.size.max(1)) as u8),
    )?;
    let stage = tempfile::tempdir_in(root).map_err(|error| error.to_string())?;
    extract(&root.join(dist.archive.file), stage.path(), dist, cancel)?;
    if cancel.load(Ordering::Relaxed) {
        return Err("下载已取消 / Download cancelled".into());
    }
    // 1. 【识别运行库】【完整发布】损坏目录只在替代文件校验完成后移除，未完成目录不参与加载
    if installed.exists() {
        fs::remove_dir_all(&installed).map_err(|error| error.to_string())?;
    }
    fs::rename(stage.path(), &installed).map_err(|error| error.to_string())?;
    let _ = fs::remove_file(root.join(dist.archive.file));
    progress(100);
    Ok(())
}
