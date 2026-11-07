//! 【原版配置】【历史字段】原版 captureHistory 与原生 history 使用同一记录策略

use super::values;
use serde_json::{Map, Value};

/// 【原版配置】【历史转换】按 root 返回兼容历史对象，输入错误字段沿用默认值，不改写原文件
pub(in crate::config) fn section(root: &Value) -> Option<Value> {
    let source = values::first(root, &["history", "captureHistory"])?;
    let mut result = Map::new();
    if let Some(enabled) = values::boolean(source)
        .or_else(|| values::first(source, &["enabled", "enable"]).and_then(values::boolean))
    {
        result.insert("enabled".into(), enabled.into());
    }
    if let Some(enabled) =
        values::first(source, &["index_annotations", "indexAnnotations"]).and_then(values::boolean)
    {
        result.insert("index_annotations".into(), enabled.into());
    }
    for (key, aliases, minimum, maximum) in [
        (
            "limit",
            &["limit", "maxItems", "maxCount", "count"][..],
            1,
            200,
        ),
        (
            "max_size_mib",
            &["max_size_mib", "maxSizeMiB", "maxSizeMib", "maxSizeMB"][..],
            1,
            4096,
        ),
    ] {
        if let Some(value) = values::first(source, aliases).and_then(values::integer) {
            result.insert(key.into(), value.clamp(minimum, maximum).into());
        }
    }
    if let Some(directory) = values::text(source, &["directory", "path"]) {
        result.insert("directory".into(), directory.into());
    }
    Some(result.into())
}
