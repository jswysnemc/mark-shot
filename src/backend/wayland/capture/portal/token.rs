//! 【Portal捕获】【恢复令牌】只保存已成功捕获的桌面授权，兼容旧目录

use std::{fs, path::PathBuf};

/// 【Portal捕获】【令牌路径】返回当前状态目录中的恢复令牌路径，无参数
fn token_path() -> Option<PathBuf> {
    Some(dirs::state_dir()?.join("mark-shot").join("token"))
}

/// 【Portal捕获】【旧令牌路径】返回旧配置目录中的恢复令牌路径，无参数
fn legacy_token_path() -> Option<PathBuf> {
    Some(dirs::config_dir()?.join("mark-shot").join("token"))
}

/// 【Portal捕获】【读取授权】优先读取当前状态目录，再兼容旧目录，返回已有恢复令牌，无参数
pub(super) fn read_token() -> Option<String> {
    [token_path(), legacy_token_path()]
        .into_iter()
        .flatten()
        .find_map(|path| fs::read_to_string(path).ok())
}

/// 【Portal捕获】【保存授权】将成功捕获的token保存到状态目录并清除旧副本，无返回值
pub(super) fn write_token(token: &str) {
    let Some(path) = token_path() else { return };
    let written = path
        .parent()
        .map_or(Ok(()), fs::create_dir_all)
        .and_then(|()| fs::write(&path, token));
    match written {
        Ok(()) => {
            if let Some(legacy) = legacy_token_path() {
                let _ = fs::remove_file(legacy);
            }
        }
        Err(e) => log::error!("【Portal捕获】【令牌保存】{}: {e}", path.display()),
    }
}

/// 【Portal捕获】【失效清理】清除两个目录中的失效恢复令牌，无参数、无返回值
pub(super) fn forget_token() {
    for path in [token_path(), legacy_token_path()].into_iter().flatten() {
        let _ = fs::remove_file(path);
    }
}
