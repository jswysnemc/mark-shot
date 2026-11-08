//! 【设置快捷键】【共用录入】工具栏和快捷键列表共用格式化、显示及冲突检查

use crate::{backend::window::KeyInput, keys::Key};
use serde_json::Value;

/// 【设置快捷键】【录入文本】按 input 返回可存储按键组合，纯修饰键事件返回空值
pub(super) fn chord(input: &KeyInput) -> Option<String> {
    let key = input.key?;
    let mut text = String::new();
    for (enabled, name) in [
        (input.mods.ctrl, "Ctrl+"),
        (input.mods.alt, "Alt+"),
        (input.mods.shift, "Shift+"),
        (input.mods.logo, "Super+"),
    ] {
        if enabled {
            text.push_str(name);
        }
    }
    text.push_str(&match key {
        Key::Char(character) => character.to_uppercase().to_string(),
        Key::F(number) => format!("F{number}"),
        other => format!("{other:?}"),
    });
    crate::keys::parse(&text).ok().map(|_| text)
}

/// 【设置快捷键】【冲突查询】按 values、key 和 value 检查其他动作占用，返回冲突配置键；空值表示禁用
pub(super) fn conflict(values: &Value, key: &str, value: &Value) -> Option<&'static str> {
    let proposed = match value {
        Value::String(text) => vec![text.as_str()],
        Value::Array(list) => list.iter().filter_map(Value::as_str).collect(),
        _ => return None,
    };
    let chords: Vec<_> = proposed
        .into_iter()
        .filter_map(|text| crate::keys::parse(text).ok())
        .collect();
    crate::keys::ACTIONS
        .iter()
        .find(|(other, _, defaults)| {
            if *other == key {
                return false;
            }
            let bindings = match &values["keys"][*other] {
                Value::String(text) => vec![text.as_str()],
                Value::Array(list) => list.iter().filter_map(Value::as_str).collect(),
                _ => defaults.to_vec(),
            };
            bindings
                .iter()
                .filter_map(|text| crate::keys::parse(text).ok())
                .any(|chord| chords.contains(&chord))
        })
        .map(|(key, _, _)| *key)
}

/// 【设置快捷键】【值显示】按 values、key 返回全部有效组合，没有配置时使用内置绑定
pub(super) fn text(values: &Value, key: &str) -> String {
    match &values["keys"][key] {
        Value::String(text) => text.clone(),
        Value::Array(list) => list
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(" / "),
        _ => defaults(key)
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(" / "),
    }
}

/// 【设置快捷键】【默认绑定】按 key 返回动作默认组合数组，未注册动作返回空数组
pub(super) fn defaults(key: &str) -> Vec<Value> {
    crate::keys::ACTIONS
        .iter()
        .find(|(name, _, _)| *name == key)
        .map_or_else(Vec::new, |(_, _, values)| {
            values.iter().map(|text| Value::from(*text)).collect()
        })
}
