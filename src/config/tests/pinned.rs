//! 【贴图配置回归】【自动划词】验证默认关闭、设置写入与旧格式解析保持一致

use super::super::*;

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
