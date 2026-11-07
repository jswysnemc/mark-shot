//! 【原版配置回归】【格式与字段】启动和单项保存读取相同 JSON，不访问用户配置

use super::*;
use serde_json::{Value, json};

/// 【原版配置回归】【固定内容】返回包含原版字段及未知厂商数据的配置，无参数，不包含真实用户凭据
fn fixture() -> Value {
    json!({
        "ui": {"theme": "dark", "language": "English"},
        "capture": {"includeCursor": true, "regionDetection": {"enabled": false}, "selectionLoupe": {"enabled": false}},
        "windowDetection": {"enabled": false},
        "captureHistory": {"enabled": false, "limit": 23},
        "pinnedWindow": {"alwaysOnTop": false, "border": false, "borderWidth": 6, "borderColor": "#123456"},
        "annotation": {"defaultTool": "pen", "fullscreenDefaultTool": "ruler", "fileDefaultTool": "rectangle", "defaultColor": "#E95544"},
        "translation": {"targetLanguage": "English", "timeoutMs": 65000, "apiKey": "local-test-value", "vendor": {"null": null, "list": [1, 2, 3]}},
        "export": {"imageFrame": false},
        "unknown": {"text": "保留其他字段", "array": [false, null, {"number": 3.5}]}
    })
}

/// 【原版配置回归】【启动解析】原版主题、历史、捕获、贴图和颜色字段真实进入启动配置，无参数、无返回值
#[test]
fn original_json_settings_are_loaded_at_startup() {
    let config = loading::parse(&fixture().to_string(), "private-config.json");
    assert_eq!(config.ui.theme, crate::theme::Mode::Dark);
    assert!(!config.history.enabled);
    assert_eq!(config.history.limit, 23);
    assert!(!config.region_detection.enabled && !config.window_detection.enabled);
    assert!(!config.pinned.always_on_top && !config.pinned.border_enabled);
    assert_eq!(config.pinned.border_width, 6.0);
    assert_eq!(config.pinned.border_color, Rgba(18, 52, 86, 255));
    assert_eq!(config.tools.color, Rgba(233, 85, 68, 255));
}

/// 【原版配置回归】【主题更新】主题切换只更新原版 ui.theme，文件继续保持 JSON，未知字段与翻译数据不变，无参数、无返回值
#[test]
fn theme_updates_keep_json_and_unrelated_fields() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    let before = fixture();
    std::fs::write(&path, before.to_string()).unwrap();
    interface_theme::save_at(&path, crate::theme::Mode::Light).unwrap();
    let after: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(after["ui"]["theme"], "light");
    assert_eq!(after["ui"]["language"], before["ui"]["language"]);
    for key in ["translation", "captureHistory", "unknown"] {
        assert_eq!(after[key], before[key]);
    }
}

/// 【原版配置回归】【翻译更新】保存语言继续更新原版 targetLanguage，厂商参数与其他业务字段不变，无参数、无返回值
#[test]
fn target_updates_keep_json_vendor_data_and_legacy_key() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    let before = fixture();
    std::fs::write(&path, before.to_string()).unwrap();
    translation::save_target_at(&path, "Japanese").unwrap();
    let after: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(after["translation"]["targetLanguage"], "Japanese");
    assert!(after["translation"].get("target_language").is_none());
    for key in ["apiKey", "vendor"] {
        assert_eq!(after["translation"][key], before["translation"][key]);
    }
    assert_eq!(after["unknown"], before["unknown"]);
}

/// 【原版配置回归】【历史更新】原版 captureHistory 开关与数量可单独更新，不生成另一个历史分组，无参数、无返回值
#[test]
fn history_updates_use_capture_history_and_preserve_other_sections() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    let before = fixture();
    std::fs::write(&path, before.to_string()).unwrap();
    let updated = history::update_at(&path, history::Update::Enabled(true)).unwrap();
    assert!(updated.enabled && updated.limit == 23);
    history::update_at(&path, history::Update::Limit(37)).unwrap();
    let after: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(after["captureHistory"]["limit"], 37);
    assert!(after.get("history").is_none());
    assert_eq!(after["translation"], before["translation"]);
    assert_eq!(after["unknown"], before["unknown"]);
}
