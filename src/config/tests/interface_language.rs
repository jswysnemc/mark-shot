use super::*;

/// 【界面语言测试】【兼容字段】对照原版新键、旧键、未知模式及错误类型优先级，不修改传入文件，无参数、无返回值
#[test]
fn json_and_toml_follow_the_original_language_key_precedence() {
    let directory = tempfile::tempdir().unwrap();
    for (json, toml, expected) in [
        (
            r#"{"ui":{"language":"zh-CN"},"language":"english"}"#,
            "language='english'\n[ui]\nlanguage='zh-CN'",
            Mode::Chinese,
        ),
        (
            r#"{"ui":{"language":"system"},"language":"chinese"}"#,
            "language='chinese'\n[ui]\nlanguage='system'",
            Mode::System,
        ),
        (
            r#"{"ui":{"language":"unknown"},"language":"chinese"}"#,
            "language='chinese'\n[ui]\nlanguage='unknown'",
            Mode::System,
        ),
        (
            r#"{"ui":{"language":" "},"language":"chinese"}"#,
            "language='chinese'\n[ui]\nlanguage=' '",
            Mode::Chinese,
        ),
        (
            r#"{"ui":{"language":12},"language":"chinese"}"#,
            "language='chinese'\n[ui]\nlanguage=12",
            Mode::Chinese,
        ),
        (
            r#"{"language":"english"}"#,
            "language='english'",
            Mode::English,
        ),
        (r#"{}"#, "", Mode::System),
    ] {
        for (extension, text) in [("json", json), ("toml", toml)] {
            let path = directory.path().join(format!("config.{extension}"));
            std::fs::write(&path, text).unwrap();
            assert_eq!(read_at(&path).unwrap(), expected);
            assert_eq!(std::fs::read_to_string(path).unwrap(), text);
        }
    }
}

/// 【界面语言测试】【读取保护】非法文本、链接、硬链接和超大文件不能进入语言读取，无参数、无返回值
#[test]
fn invalid_or_unsafe_configuration_is_not_read_or_rewritten() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    std::fs::write(&path, "{").unwrap();
    assert!(read_at(&path).is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "{");
    std::fs::write(&path, r#"{"ui":{"language":"chinese"}}"#).unwrap();
    let link = directory.path().join("link.json");
    crate::utils::test_files::link_file(&path, &link).unwrap();
    assert!(read_at(&link).is_err());
    std::fs::hard_link(&path, directory.path().join("hard.json")).unwrap();
    assert!(read_at(&path).is_err());
    std::fs::remove_file(directory.path().join("hard.json")).unwrap();
    std::fs::write(&path, " ".repeat(1024 * 1024 + 1)).unwrap();
    assert!(read_at(&path).is_err());
}

/// 【界面语言测试】【启动解析】原生完整配置接受新字段及旧顶层键，缺失新键不覆盖旧键，无参数、无返回值
#[test]
fn native_config_accepts_the_current_and_legacy_language_fields() {
    let config: crate::config::Config = toml::from_str("language='chinese'").unwrap();
    assert_eq!(config.ui.mode(config.language.as_deref()), Mode::Chinese);
    let config: crate::config::Config =
        toml::from_str("language='chinese'\n[ui]\nlanguage='english'").unwrap();
    assert_eq!(config.ui.mode(config.language.as_deref()), Mode::English);
    assert_eq!(crate::config::Config::default().ui.mode(None), Mode::System);
}
