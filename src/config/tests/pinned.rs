//! 【贴图配置回归】【自动划词】验证默认关闭、设置写入与旧格式解析保持一致

use super::super::*;

/// 【贴图配置回归】【剪贴板比例】无参数，验证默认 auto 与字符串、数字 1.0 的解析结果，无返回值
#[test]
fn clipboard_image_scale_parses_named_and_numeric_values() {
    assert_eq!(
        Config::default().pinned.clipboard_image_scale,
        ClipboardImageScale::Auto
    );
    for (text, expected) in [
        (
            "[pinned]\nclipboard_image_scale = \"auto\"\n",
            ClipboardImageScale::Auto,
        ),
        (
            "[pinned]\nclipboard_image_scale = \"system\"\n",
            ClipboardImageScale::System,
        ),
        (
            "[pinned]\nclipboard_image_scale = \"1.0\"\n",
            ClipboardImageScale::One,
        ),
        (
            "[pinned]\nclipboard_image_scale = 1.0\n",
            ClipboardImageScale::One,
        ),
    ] {
        assert_eq!(
            loading::parse(text, "config.toml")
                .pinned
                .clipboard_image_scale,
            expected
        );
    }
}

/// 【贴图配置回归】【设置交接】无参数，验证 TOML、JSON 和设置保存都启用同一启动字段，无返回值
#[test]
fn automatic_text_selection_is_opt_in_and_survives_settings_roundtrip() {
    assert!(!Config::default().pinned.auto_text_selection);
    for (text, name) in [
        ("[pinned]\nauto_text_selection = true\n", "config.toml"),
        (
            r#"{"pinnedWindow":{"autoTextSelection":true}}"#,
            "config.json",
        ),
    ] {
        assert!(loading::parse(text, name).pinned.auto_text_selection);
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join(name);
        std::fs::write(&path, text).unwrap();
        settings_store::write(
            &path,
            &[("pinned.auto_text_selection".into(), false.into())],
        )
        .unwrap();
        assert!(
            !loading::parse(&std::fs::read_to_string(path).unwrap(), name)
                .pinned
                .auto_text_selection
        );
    }
    let entry = settings::ENTRIES
        .iter()
        .find(|entry| entry.path() == "pinned.auto_text_selection")
        .unwrap();
    assert_eq!(entry.page, settings::Page::Pinned);
    assert_eq!(entry.default_value(), serde_json::Value::Bool(false));
}
