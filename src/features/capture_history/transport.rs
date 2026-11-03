//! 【截图历史】【进程交接】独立窗口与安全原图重编辑

use super::Store;
use crate::{AppResult, app::Launch, config::history};
use std::io::{self, Write};
use std::{fs::File, os::fd::AsRawFd};

pub const READY_ARG: &str = "--history-ready";

/// 【截图历史】【窗口启动】启动同一构建的历史窗口并等待首帧；成功后截图覆盖层可以关闭
pub fn spawn() -> AppResult {
    let mut args = vec![READY_ARG];
    if !crate::config::get().notifications.enabled {
        args.push("--nonotify");
    }
    crate::utils::spawn_self_ready(&args, &[], std::time::Duration::from_secs(3))
}

/// 【截图历史】【重编辑启动】读取 store 中合法 name 并交接 PNG 快照；子进程不会重新打开可能变化的历史路径
pub(super) fn spawn_edit(store: &Store, name: &str) -> io::Result<()> {
    if super::identity::timestamp(name).is_none() {
        return Err(io::Error::other("invalid history identity"));
    }
    let (png, _) = store.read(name)?;
    let mut args = vec!["--history-edit-snapshot"];
    if !crate::config::get().notifications.enabled {
        args.push("--nonotify");
    }
    crate::utils::spawn_self(&args, &png).map_err(|error| io::Error::other(error.to_string()))
}

/// 【截图历史】【编辑快照】读取合法 name 到私有临时文件；返回保持文件生命周期的句柄与图片编辑参数
pub fn edit_snapshot(name: &str) -> io::Result<(File, Launch)> {
    let settings = history::current();
    let store = Store::new(settings.path()?, settings.byte_limit());
    let (png, _) = store.read(name)?;
    materialize(&png)
}

/// 【截图历史】【编辑快照接收】从标准输入读取有界 PNG 并校验；返回私有临时文件与编辑参数
pub fn edit_from_stdin() -> io::Result<(File, Launch)> {
    use std::io::Read;
    let limit = crate::config::history::current()
        .byte_limit()
        .min(super::png_metadata::PNG_BYTE_LIMIT);
    let mut png = Vec::new();
    std::io::stdin()
        .lock()
        .take(limit + 1)
        .read_to_end(&mut png)?;
    if png.len() as u64 > limit {
        return Err(io::Error::other("history snapshot is too large"));
    }
    super::png_metadata::inspect(&mut std::io::Cursor::new(&png))?;
    super::png_metadata::verify(std::io::Cursor::new(&png))?;
    materialize(&png)
}

/// 【截图历史】【快照落地】把已验证 png 写入私有临时文件；返回保持文件生命周期的句柄和图片编辑参数
fn materialize(png: &[u8]) -> io::Result<(File, Launch)> {
    use std::os::unix::fs::PermissionsExt;
    // 1. 【截图历史】【匿名快照】文件没有目录名称，即使进程异常退出也不留下额外截图文件
    let mut file = tempfile::tempfile()?;
    file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    file.write_all(png)?;
    file.flush()?;
    let launch = Launch {
        image: Some(std::path::PathBuf::from(format!(
            "/proc/self/fd/{}",
            file.as_raw_fd()
        ))),
        one_monitor: true,
        ..Launch::default()
    };
    Ok((file, launch))
}

#[cfg(test)]
mod tests {
    use super::*;
    /// 【截图历史测试】【匿名交接】快照只有文件描述符，保持私有权限且可通过编辑参数读取原始 PNG
    #[test]
    fn editing_snapshots_have_no_persistent_filename() {
        use std::os::unix::fs::MetadataExt;
        let png = super::super::tests::png(17, 9);
        let (file, launch) = materialize(&png).unwrap();
        assert_eq!(file.metadata().unwrap().nlink(), 0);
        assert_eq!(file.metadata().unwrap().mode() & 0o777, 0o600);
        assert_eq!(std::fs::read(launch.image.unwrap()).unwrap(), png);
    }
}
