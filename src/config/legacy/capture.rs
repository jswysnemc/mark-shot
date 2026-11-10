//! 【原版配置】【捕获字段】图像识别、窗口识别、鼠标包含和选区放大区分别读取

use super::values;
use serde_json::{Value, json};

/// 【原版配置】【捕获对象】按 root 返回首个非空捕获对象，缺失时返回空值引用
pub(crate) fn capture_key(root: &Value) -> &'static str {
    ["capture", "screenshot", "screenCapture"]
        .into_iter()
        .find(|key| {
            root.get(*key)
                .is_some_and(|value| value.as_object().is_some_and(|object| !object.is_empty()))
        })
        .unwrap_or("capture")
}

/// 【原版配置】【捕获对象】按 root 返回读写共用路径中的对象，缺失时返回空值
fn capture(root: &Value) -> &Value {
    &root[capture_key(root)]
}

/// 【原版配置】【截图策略】按 root 独立转换已支持的行为字段；非法字段单独回退，不丢失其他有效配置
pub(super) fn settings(root: &Value) -> Value {
    let source = capture(root);
    let mut result = serde_json::to_value(super::super::capture::Capture::default()).unwrap();
    for (key, aliases) in [
        (
            "freeze_scope",
            &["freeze_scope", "freezeScope", "freezeDisplayScope"][..],
        ),
        (
            "hide_own_windows",
            &[
                "hide_own_windows",
                "hideOwnWindows",
                "hideOwnWindowsDuringCapture",
            ],
        ),
        (
            "double_click_action",
            &["double_click_action", "doubleClickAction"],
        ),
        (
            "middle_click_action",
            &["middle_click_action", "middleClickAction"],
        ),
        ("toolbar_position", &["toolbar_position", "toolbarPosition"]),
        ("kde_kwin_screenshot", &["kde_kwin_screenshot"]),
    ] {
        let value = values::first(source, aliases).or_else(|| {
            (key == "freeze_scope")
                .then(|| values::first(root, aliases))
                .flatten()
        });
        let value = if key == "kde_kwin_screenshot" {
            value.or_else(|| source.pointer("/wayland/kde/kwinScreenshot/enabled"))
        } else {
            value
        };
        let Some(value) = value else { continue };
        let value = if matches!(key, "hide_own_windows" | "kde_kwin_screenshot") {
            values::boolean(value).map(Value::Bool)
        } else if value == &Value::Bool(false) && key.ends_with("click_action") {
            Some("none".into())
        } else {
            Some(value.clone())
        };
        if let Some(value) = value {
            let mut candidate = result.clone();
            candidate[key] = value;
            if let Ok(config) = serde_json::from_value::<super::super::capture::Capture>(candidate)
            {
                result = serde_json::to_value(config).unwrap();
            }
        }
    }
    result
}

/// 【原版配置】【识别开关】按 root 与 image 返回独立识别对象，错误值使用原版开启默认，不创建任务
pub(super) fn detection(root: &Value, image: bool) -> Value {
    let (native, legacy) = if image {
        ("region_detection", "regionDetection")
    } else {
        ("window_detection", "windowDetection")
    };
    let value = root
        .get(native)
        .or_else(|| root.get(legacy))
        .or_else(|| {
            image
                .then(|| capture(root).get("regionDetection"))
                .flatten()
        })
        .unwrap_or(&Value::Null);
    json!({"enabled": values::boolean(value).or_else(|| values::first(value, &["enabled", "enable"]).and_then(values::boolean)).unwrap_or(true)})
}

/// 【原版配置】【鼠标开关】按 root 返回鼠标包含设置，错误或缺失时返回关闭
pub(super) fn cursor(root: &Value) -> bool {
    let nested = values::first(
        capture(root),
        &[
            "includeCursor",
            "includeMouse",
            "includePointer",
            "showCursor",
            "showMouse",
            "captureCursor",
            "captureMouse",
        ],
    )
    .filter(|value| !value.is_null());
    nested
        .or_else(|| {
            values::first(
                root,
                &[
                    "captureIncludeCursor",
                    "captureIncludeMouse",
                    "includeCursor",
                    "includeMouse",
                    "includePointer",
                ],
            )
        })
        .and_then(values::boolean)
        .unwrap_or(false)
}

/// 【原版配置】【放大区开关】按 root 返回选区放大区设置，取色阶段保留独立职责，错误或缺失时返回关闭
pub(super) fn loupe(root: &Value) -> bool {
    let capture = capture(root);
    let loupe = ["selectionLoupe", "loupe"]
        .iter()
        .find_map(|key| {
            capture
                .get(*key)
                .filter(|value| value.as_object().is_some_and(|object| !object.is_empty()))
        })
        .unwrap_or(&Value::Null);
    let nested = values::first(loupe, &["enabled", "enable"]).filter(|value| !value.is_null());
    let direct = values::first(
        capture,
        &["selectionLoupe", "selectionLoupeEnabled", "loupeEnabled"],
    )
    .filter(|value| !value.is_null() && !value.is_object());
    nested
        .or(direct)
        .or_else(|| values::first(root, &["selectionLoupeEnabled", "captureSelectionLoupe"]))
        .and_then(values::boolean)
        .unwrap_or(false)
}
