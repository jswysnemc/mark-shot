//! 【打开方式】【应用发现】按 Desktop Entry 文件读取图片应用，首次打开面板前不扫描目录

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesktopApp {
    pub name: String,
    pub desktop_path: PathBuf,
    pub exec: String,
    pub icon: String,
}

/// 【打开方式】【条目解析】按 text 和 path 返回可见图片应用；忽略动作分组、隐藏条目和缺少命令的条目
pub fn parse(text: &str, path: &Path) -> Option<DesktopApp> {
    let mut fields = BTreeMap::new();
    let mut entry = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') && line.ends_with(']') {
            entry = line == "[Desktop Entry]";
        } else if entry
            && !line.starts_with('#')
            && let Some((key, value)) = line.split_once('=')
        {
            fields.entry(key.trim()).or_insert(value.trim());
        }
    }
    let value = |key| fields.get(key).copied().unwrap_or_default();
    let boolean = |key| matches!(value(key).to_ascii_lowercase().as_str(), "true" | "1");
    if value("Type") != "Application"
        || boolean("Hidden")
        || boolean("NoDisplay")
        || !value("MimeType")
            .split(';')
            .any(|mime| mime.trim().to_ascii_lowercase().starts_with("image/"))
        || value("Name").is_empty()
        || value("Exec").is_empty()
    {
        return None;
    }
    Some(DesktopApp {
        name: value("Name").to_owned(),
        desktop_path: path.to_owned(),
        exec: value("Exec").to_owned(),
        icon: value("Icon").to_owned(),
    })
}

/// 【打开方式】【搜索目录】返回用户和系统应用目录，遵循 XDG_DATA_HOME、XDG_DATA_DIRS 并去除重复路径，无参数
pub fn search_dirs() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(root) = dirs::data_dir() {
        roots.push(root);
    }
    roots.extend(std::env::split_paths(
        &std::env::var_os("XDG_DATA_DIRS")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "/usr/local/share:/usr/share".into()),
    ));
    let mut result = Vec::new();
    for root in roots.into_iter().filter(|root| root.is_absolute()) {
        let path = root.join("applications");
        if !result.contains(&path) {
            result.push(path);
        }
    }
    result
}

/// 【打开方式】【目录枚举】从 directories 递归读取桌面文件，返回按名称排序的应用；拒绝链接目录环，不写入参考目录
pub fn discover(directories: &[PathBuf]) -> Vec<DesktopApp> {
    let mut pending = directories.to_vec();
    let mut seen = std::collections::HashSet::new();
    let mut apps = Vec::new();
    while let Some(directory) = pending.pop() {
        let Ok(entries) = fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            if kind.is_dir() {
                pending.push(path);
                continue;
            }
            if path
                .extension()
                .is_none_or(|extension| extension != "desktop")
                || !seen.insert(path.clone())
            {
                continue;
            }
            // 1. 【打开方式】【有界读取】畸形桌面文件不能产生无界分配，正常链接文件仍支持系统应用入口
            let Ok(file) = fs::File::open(&path) else {
                continue;
            };
            let Ok(metadata) = file.metadata() else {
                continue;
            };
            if !metadata.is_file() || metadata.len() > 1024 * 1024 {
                continue;
            }
            let mut text = String::new();
            use std::io::Read;
            if file.take(1024 * 1024 + 1).read_to_string(&mut text).is_ok()
                && text.len() <= 1024 * 1024
                && let Some(app) = parse(&text, &path)
            {
                apps.push(app);
            }
        }
    }
    apps.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.desktop_path.cmp(&right.desktop_path))
    });
    apps
}
