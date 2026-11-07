//! 【原版配置】【基础值】沿用 Qt 布尔、整数、颜色和字段选择规则

use crate::theme::Rgba;
use serde_json::Value;

/// 【原版配置】【字段顺序】按 keys 返回 root 首个已定义值，空值不跳过，返回借用值
pub(super) fn first<'a>(root: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().find_map(|key| root.get(*key))
}

/// 【原版配置】【字符串候选】按 keys 返回首个非空字符串，返回去掉首尾空白的借用值
pub(super) fn text<'a>(root: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .filter_map(|key| root.get(*key).and_then(Value::as_str))
        .map(str::trim)
        .find(|value| !value.is_empty())
}

/// 【原版配置】【布尔转换】按 value 返回 Qt 兼容布尔值，错误类型返回空值
pub(super) fn boolean(value: &Value) -> Option<bool> {
    match value {
        Value::Bool(value) => Some(*value),
        Value::Number(value) => value.as_f64().map(|value| value.abs() > 1e-12),
        Value::String(value) => match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" | "enabled" => Some(true),
            "0" | "false" | "no" | "off" | "disabled" => Some(false),
            _ => None,
        },
        _ => None,
    }
}

/// 【原版配置】【整数转换】按 value 返回整数字符串或 Qt 整数值，非整数 JSON 数字按原版返回零
pub(super) fn integer(value: &Value) -> Option<i32> {
    match value {
        Value::String(value) => value.trim().parse().ok(),
        Value::Number(value) => Some(
            value
                .as_f64()
                .filter(|value| {
                    value.fract() == 0.0 && *value >= i32::MIN as f64 && *value <= i32::MAX as f64
                })
                .map_or(0, |value| value as i32),
        ),
        _ => None,
    }
}

/// 【原版配置】【实数转换】按 value 返回有限数字或数字字符串，非有限值及错误类型返回空值
pub(super) fn real(value: &Value) -> Option<f64> {
    match value {
        Value::Number(value) => value.as_f64(),
        Value::String(value) => value.trim().parse().ok(),
        _ => None,
    }
    .filter(|value| value.is_finite())
}

/// 【原版配置】【颜色文本】按 value 读取原版十六进制或 SVG 颜色名，返回 RGBA，拒绝非十六进制通道字符
fn color_text(value: &str) -> Option<Rgba> {
    let value = value.trim();
    let value = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .unwrap_or(value);
    let hex = value.strip_prefix('#').or_else(|| {
        (matches!(value.len(), 6 | 8) && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
            .then_some(value)
    });
    if let Some(hex) = hex {
        if !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return None;
        }
        if matches!(hex.len(), 9 | 12) {
            let digits = hex.len() / 3;
            let maximum = (1_u32 << (digits * 4)) - 1;
            let channel = |offset| {
                u32::from_str_radix(&hex[offset..offset + digits], 16)
                    .ok()
                    .map(|value| ((value * 255 + maximum / 2) / maximum) as u8)
            };
            return Some(Rgba(
                channel(0)?,
                channel(digits)?,
                channel(digits * 2)?,
                255,
            ));
        }
        if !matches!(hex.len(), 3 | 6 | 8) {
            return None;
        }
        let parsed: svgtypes::Color = format!("#{hex}").parse().ok()?;
        return Some(Rgba(parsed.red, parsed.green, parsed.blue, parsed.alpha));
    }
    if !value.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        return None;
    }
    let parsed: svgtypes::Color = value.to_ascii_lowercase().parse().ok()?;
    Some(Rgba(parsed.red, parsed.green, parsed.blue, parsed.alpha))
}

/// 【原版配置】【颜色对象】按 value 返回颜色文本、嵌套颜色或 RGBA 通道，通道夹紧到零至二百五十五
pub(super) fn color(value: &Value) -> Option<Rgba> {
    if let Some(value) = value.as_str() {
        return color_text(value);
    }
    if let Some(value) =
        first(value, &["value", "color", "hex", "format", "formats"]).and_then(Value::as_str)
    {
        return color_text(value);
    }
    let channel = |keys: &[&str]| {
        first(value, keys)
            .and_then(integer)
            .map(|number| number.clamp(0, 255) as u8)
    };
    Some(Rgba(
        channel(&["r", "red"])?,
        channel(&["g", "green"])?,
        channel(&["b", "blue"])?,
        channel(&["a", "alpha"]).unwrap_or(255),
    ))
}
