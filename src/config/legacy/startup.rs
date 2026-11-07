//! 【原版配置】【启动字段】默认工具顺序、颜色与调试选项转换，敏感正文不参与日志

use super::{capture, values};
use serde_json::{Map, Value};

/// 【原版配置】【工具候选】按 sources 顺序返回首个非空且有效的工具值；首个非空错误名称不回退到更低优先级
fn tool(sources: &[(&Value, &[&str])]) -> Option<Value> {
    let name = sources
        .iter()
        .find_map(|(root, keys)| values::text(root, keys))?;
    crate::tools::names::parse(name).ok().map(|_| name.into())
}

/// 【原版配置】【默认工具】按 root 返回原版共同、区域、全屏和文件参数，原生 startup 最后覆盖，返回启动对象及明确颜色
pub(super) fn section(root: &Value) -> (Value, Option<crate::theme::Rgba>) {
    let annotation = root.get("annotation").unwrap_or(&Value::Null);
    let nested = annotation.get("defaultTools").unwrap_or(&Value::Null);
    let outer = root.get("defaultTools").unwrap_or(&Value::Null);
    let mut result = Map::new();
    for (key, value) in [
        (
            "default_tool",
            tool(&[
                (annotation, &["defaultTool", "defaultAnnotationTool"]),
                (root, &["defaultTool", "defaultAnnotationTool"]),
            ]),
        ),
        (
            "normal_tool",
            tool(&[
                (nested, &["normal", "region", "selection"]),
                (outer, &["normal", "region", "selection"]),
                (
                    annotation,
                    &[
                        "normalDefaultTool",
                        "regionDefaultTool",
                        "selectionDefaultTool",
                    ],
                ),
                (
                    root,
                    &[
                        "normalDefaultTool",
                        "regionDefaultTool",
                        "selectionDefaultTool",
                    ],
                ),
            ]),
        ),
        (
            "fullscreen_default_tool",
            tool(&[
                (nested, &["fullscreen", "fullScreen", "full-screen"]),
                (outer, &["fullscreen", "fullScreen", "full-screen"]),
                (
                    annotation,
                    &[
                        "fullscreenDefaultTool",
                        "fullScreenDefaultTool",
                        "defaultFullscreenTool",
                    ],
                ),
                (
                    root,
                    &[
                        "fullscreenDefaultTool",
                        "fullScreenDefaultTool",
                        "defaultFullscreenTool",
                    ],
                ),
            ]),
        ),
        (
            "file_default_tool",
            tool(&[
                (
                    annotation,
                    &["fileDefaultTool", "imageDefaultTool", "openFileDefaultTool"],
                ),
                (
                    root,
                    &["fileDefaultTool", "imageDefaultTool", "openFileDefaultTool"],
                ),
                (nested, &["file", "image", "openFile"]),
                (outer, &["file", "image", "openFile"]),
            ]),
        ),
    ] {
        if let Some(value) = value {
            result.insert(key.into(), value);
        }
    }
    let color = values::first(annotation, &["defaultColor", "defaultAnnotationColor"])
        .or_else(|| values::first(root, &["defaultColor", "defaultAnnotationColor"]))
        .and_then(values::color);
    if let Some(color) = color {
        result.insert(
            "default_color".into(),
            serde_json::to_value(color).expect("RGBA serializes to a string"),
        );
    }
    result.insert("include_cursor".into(), capture::cursor(root).into());
    result.insert("selection_loupe".into(), capture::loupe(root).into());
    if let Some(native) = root.get("startup").and_then(Value::as_object) {
        result.extend(native.clone());
    }
    (result.into(), color)
}

/// 【原版配置】【调试字段】按 root 返回日志参数，原生 log 最后覆盖，不读取环境变量或输出参数正文
pub(super) fn log(root: &Value) -> Value {
    let source = root.get("debug").unwrap_or(&Value::Null);
    let mut result = Map::new();
    if let Some(enabled) = values::boolean(source)
        .or_else(|| values::first(source, &["enabled", "enable", "on"]).and_then(values::boolean))
    {
        result.insert("level".into(), if enabled { "info" } else { "off" }.into());
    }
    let path = values::text(root, &["debugLogPath", "debugLog", "logPath"])
        .or_else(|| values::text(source, &["logPath", "path", "file", "logFile"]));
    if let Some(path) = path {
        result.insert("path".into(), path.into());
    }
    if let Some(native) = root.get("log").and_then(Value::as_object) {
        result.extend(native.clone());
    }
    result.into()
}
