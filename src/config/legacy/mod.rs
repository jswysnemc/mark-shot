//! 【原版配置】【启动转换】仅转换运行时字段，保存始终基于原始 JSON 对象

use serde_json::Value;
use std::{io, path::Path};

pub(crate) mod capture;
pub(super) mod history;
mod pinned;
mod shortcuts;
mod startup;
mod values;

/// 【原版配置】【完整解析】按 text 返回原生运行参数，明确配置保留，原始厂商数据不重写为另一格式
pub(super) fn decode(text: &str) -> io::Result<super::Config> {
    let root = super::document::root(Path::new("config.json"), text)?;
    let mut normalized = root
        .as_object()
        .expect("root was checked as an object")
        .clone();
    if let Some(history) = history::section(&root) {
        normalized.insert("history".into(), history);
    }
    if let Some(pinned) = pinned::section(&root) {
        normalized.insert("pinned".into(), pinned);
    }
    normalized.insert("region_detection".into(), capture::detection(&root, true));
    normalized.insert("window_detection".into(), capture::detection(&root, false));
    normalized.remove("regionDetection");
    normalized.remove("windowDetection");
    normalized.insert("keys".into(), shortcuts::section(&root));
    let (startup, color) = startup::section(&root);
    normalized.insert("startup".into(), startup);
    normalized.insert("log".into(), startup::log(&root));
    if let Some(ui) = normalized.get_mut("ui").and_then(Value::as_object_mut) {
        ui.retain(|key, value| !matches!(key.as_str(), "language" | "theme") || value.is_string());
    }
    let mut config: super::Config =
        serde_json::from_value(normalized.into()).map_err(io::Error::other)?;
    if let Some(color) = color {
        config.tools.color = color;
    }
    super::interface_theme::apply_root(&mut config, &root);
    config.resolve();
    super::loading::validate(&mut config);
    Ok(config)
}
