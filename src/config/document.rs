//! 【应用配置】【文档格式】各业务共用格式判断和 JSON 单项更新，保留未知字段

use serde::de::DeserializeOwned;
use serde_json::{Map, Value};
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

/// 【应用配置】【JSON 更新】对 text 的根对象执行 edit，返回完整 JSON；业务或格式错误不产生替换内容
pub(super) fn update_json(
    text: &str,
    edit: impl FnOnce(&mut Value) -> io::Result<()>,
) -> io::Result<String> {
    let mut root = root(Path::new("config.json"), text)?;
    edit(&mut root)?;
    Ok(serde_json::to_string_pretty(&root).map_err(io::Error::other)? + "\n")
}

/// 【应用配置】【分组更新】按 keys 优先选择已存在分组，否则创建 default；返回可修改对象，拒绝非法分组类型
pub(super) fn section<'a>(
    root: &'a mut Value,
    keys: &[&str],
    default: &str,
) -> io::Result<&'a mut Map<String, Value>> {
    let key = keys
        .iter()
        .copied()
        .find(|key| root.get(*key).is_some())
        .unwrap_or(default);
    root.as_object_mut()
        .ok_or_else(|| io::Error::other("configuration root must be an object"))?
        .entry(key)
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or_else(|| io::Error::other("configuration section must be an object"))
}
