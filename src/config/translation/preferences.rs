//! 【翻译配置】【表单持久化】在原子事务内更新字段，保留其他服务与未知配置

use serde_json::{Map, Value};
use std::{io, path::Path};

/// 【翻译配置】【表单保存】把 patch 合并到 path 的翻译配置，返回结果；自定义正文作为 JSON 字符串保存
pub(crate) fn save(path: &Path, patch: &Map<String, Value>) -> io::Result<()> {
    let mut changes = Vec::new();
    for (key, value) in patch {
        if let Some(fields) = value.as_object() {
            for (field, value) in fields {
                changes.push((format!("translation.{key}.{field}"), value.clone()));
            }
        } else {
            changes.push((format!("translation.{key}"), value.clone()));
        }
    }
    super::super::settings_store::write(path, &changes)
}
