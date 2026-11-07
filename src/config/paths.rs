//! 【应用配置】【路径选择】保持当前进程的实际配置路径，原版显式目录不回退其他目录

use std::path::{Path, PathBuf};

/// 【应用配置】【路径存在】按 path 检查目录项存在性，返回结果，不把失效链接当作缺失配置
fn present(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok()
}

/// 【应用配置】【候选选择】按 explicit、directory、config_home 和 home 返回配置路径；显式目录固定 JSON，已有原生 TOML 优先
pub(super) fn select(
    explicit: Option<PathBuf>,
    directory: Option<&str>,
    config_home: Option<PathBuf>,
    home: Option<PathBuf>,
) -> Option<PathBuf> {
    if let Some(path) = explicit {
        return Some(std::path::absolute(&path).unwrap_or(path));
    }
    if let Some(directory) = directory
        .map(str::trim)
        .filter(|directory| !directory.is_empty())
    {
        let path = Path::new(directory).join("config.json");
        return Some(std::path::absolute(&path).unwrap_or(path));
    }
    let primary = config_home.or_else(|| home.as_ref().map(|home| home.join(".config")))?;
    let native = primary.join("mark-shot/config.toml");
    if present(&native) {
        return Some(native);
    }
    for directory in [Some(primary), home.map(|home| home.join(".config"))]
        .into_iter()
        .flatten()
    {
        let path = directory.join("mark-shot/config.json");
        if present(&path) {
            return Some(path);
        }
    }
    Some(native)
}

/// 【应用配置】【当前环境】无参数，按原版显式目录和系统配置目录返回默认路径，不创建文件
pub(super) fn default_path() -> Option<PathBuf> {
    select(
        None,
        std::env::var("MARK_SHOT_CONFIG_DIR").ok().as_deref(),
        dirs::config_dir(),
        dirs::home_dir(),
    )
}
