//! 【原版配置】【贴图字段】沿用独立置顶及布尔或对象边框配置

use super::values;
use serde_json::{Map, Value};

/// 【原版配置】【贴图转换】按 root 返回原生贴图字段，边框对象优先补齐颜色和宽度，不处理原图或窗口实例
pub(super) fn section(root: &Value) -> Option<Value> {
    let source = values::first(root, &["pinned", "pinnedWindow"])?;
    let mut result = Map::new();
    if let Some(value) =
        values::first(source, &["always_on_top", "alwaysOnTop"]).and_then(values::boolean)
    {
        result.insert("always_on_top".into(), value.into());
    }
    let border = source.get("border").unwrap_or(&Value::Null);
    let enabled = values::first(source, &["border_enabled", "borderEnabled"])
        .and_then(values::boolean)
        .or_else(|| values::boolean(border))
        .or_else(|| {
            values::first(border, &["enabled", "visible", "show"]).and_then(values::boolean)
        });
    if let Some(enabled) = enabled {
        result.insert("border_enabled".into(), enabled.into());
    }
    let color = values::first(source, &["border_color", "borderColor"])
        .and_then(values::color)
        .or_else(|| {
            values::first(border, &["color", "borderColor", "value"]).and_then(values::color)
        });
    if let Some(color) = color {
        result.insert(
            "border_color".into(),
            serde_json::to_value(color).expect("RGBA serializes to a string"),
        );
    }
    let width = values::first(source, &["border_width", "borderWidth"])
        .and_then(values::real)
        .or_else(|| {
            values::first(border, &["width", "borderWidth", "size"]).and_then(values::real)
        });
    if let Some(width) = width {
        result.insert("border_width".into(), width.clamp(0.0, 16.0).into());
    }
    Some(result.into())
}
