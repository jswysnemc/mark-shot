//! 【应用配置】【文档格式】各业务共用格式判断和 JSON 单项更新，保留未知字段

use serde::de::DeserializeOwned;
use serde_json::Value;
use std::{io, path::Path};

/// 【应用配置】【格式判断】按 path 扩展名和 text 首个字符判断 JSON，返回格式标志；显式 JSON 路径不回退为 TOML
pub(super) fn is_json(path: &Path, text: &str) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
        || text.trim_start().starts_with('{')
}

/// 【应用配置】【完整对象】按 path 和 text 读取 JSON 或 TOML 根对象，返回格式或根类型错误，不修改文件
pub(super) fn root(path: &Path, text: &str) -> io::Result<Value> {
    let root = if is_json(path, text) {
        serde_json::from_str(text).map_err(io::Error::other)?
    } else {
        serde_json::to_value(toml::from_str::<toml::Value>(text).map_err(io::Error::other)?)
            .map_err(io::Error::other)?
    };
    if !root.is_object() {
        return Err(io::Error::other("configuration root must be an object"));
    }
    Ok(root)
}

/// 【应用配置】【字段解码】按 path 和 text 返回目标类型的字段，不改变兼容键优先级或文件格式
pub(super) fn decode<T: DeserializeOwned>(path: &Path, text: &str) -> io::Result<T> {
    serde_json::from_value(root(path, text)?).map_err(io::Error::other)
}
