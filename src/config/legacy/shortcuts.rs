//! 【原版配置】【快捷键分组】原版动作与工具分组转换为已有原生动作名

use crate::keys::Action;
use serde_json::{Map, Value};

/// 【原版配置】【快捷键值】按 value 判断字符串或完整字符串列表，返回是否合法，空字符串保留禁用含义
fn valid(value: &Value) -> bool {
    value.is_string()
        || value
            .as_array()
            .is_some_and(|values| values.iter().all(Value::is_string))
}

/// 【原版配置】【快捷键转换】按 root 返回实际动作键表，原生 keys 最后覆盖原版分组，不修改全局热键或未知原字段
pub(super) fn section(root: &Value) -> Value {
    let mut keys = Map::new();
    if let Some(actions) = root
        .get("shortcuts")
        .and_then(|value| value.get("actions"))
        .and_then(Value::as_object)
    {
        for (name, value) in actions {
            if valid(value) && crate::keys::ACTIONS.iter().any(|(key, _, _)| key == name) {
                keys.insert(name.clone(), value.clone());
            }
        }
    }
    if let Some(tools) = root
        .get("shortcuts")
        .and_then(|value| value.get("tools"))
        .and_then(Value::as_object)
    {
        for (name, value) in tools {
            if valid(value)
                && let Ok(tool) = crate::tools::names::parse(name)
                && let Some((key, _, _)) = crate::keys::ACTIONS
                    .iter()
                    .find(|(_, action, _)| *action == Action::Tool(tool))
            {
                keys.insert((*key).into(), value.clone());
            }
        }
    }
    if let Some(native) = root.get("keys").and_then(Value::as_object) {
        keys.extend(
            native
                .iter()
                .filter(|(_, value)| valid(value))
                .map(|(key, value)| (key.clone(), value.clone())),
        );
    }
    keys.into()
}
