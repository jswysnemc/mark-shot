//! 【截图历史】【原图关联】完整原图与完成图共用身份，成对计费和清理

use super::{identity, storage::Directory};
use std::io;

/// 【截图历史】【原图名称】按合法完成图 name 返回完整原图名称，非法身份返回空值
pub(super) fn name(name: &str) -> Option<String> {
    identity::timestamp(name)?;
    Some(format!("original-{}", name.strip_prefix("capture-")?))
}

/// 【截图历史】【原图身份】按 name 返回关联完成图名称，拒绝路径和伪造时间
pub(super) fn capture(name: &str) -> Option<String> {
    let capture = format!("capture-{}", name.strip_prefix("original-")?);
    identity::timestamp(&capture)?;
    Some(capture)
}

/// 【截图历史】【文件资格】按 name 判断是否属于完成图或完整原图，返回布尔值
pub(super) fn valid(name: &str) -> bool {
    identity::timestamp(name).is_some() || capture(name).is_some()
}

/// 【截图历史】【原图大小】按 directory 和 name 读取关联原图长度，缺失返回零
pub(super) fn bytes(directory: &Directory, name: &str) -> u64 {
    self::name(name)
        .and_then(|name| directory.read(&name).ok())
        .and_then(|file| file.metadata().ok())
        .map_or(0, |metadata| metadata.len())
}

/// 【截图历史】【成对删除】按 directory 和 name 删除完整原图及完成图，返回安全文件操作结果
pub(super) fn remove(directory: &Directory, name: &str) -> io::Result<()> {
    let source = self::name(name).ok_or_else(|| io::Error::other("invalid history identity"))?;
    directory.remove(&source)?;
    directory.remove(name)
}

/// 【截图历史】【孤立原图】按 directory 清理没有完成图的原图，返回结果；其他文件与链接保持原有策略
pub(super) fn prune_orphans(directory: &Directory) -> io::Result<()> {
    let files = directory.owned()?;
    for (name, _) in &files {
        if let Some(capture) = capture(name)
            && !files.iter().any(|(name, _)| *name == capture)
        {
            directory.remove(name)?;
        }
    }
    Ok(())
}
