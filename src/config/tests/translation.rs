//! 【翻译配置测试】【私有文件】验证厂商字段、兼容键和读取安全，不修改用户配置

use super::*;

/// 【翻译配置测试】【序列化存在性】默认超时不能凭空成为显式覆盖，用户明确配置的六十秒必须保留，无参数、无返回值
#[test]
fn serialized_timeout_preserves_absent_and_explicit_configuration() {
    for text in ["{}", "{\"timeoutMs\":60000}", "{\"timeout_ms\":45000}"] {
        let original: Translation = serde_json::from_str(text).unwrap();
        let encoded = serde_json::to_value(&original).unwrap();
        let decoded: Translation = serde_json::from_value(encoded.clone()).unwrap();
        assert_eq!(decoded.timeout_explicit, original.timeout_explicit);
        assert_eq!(
            encoded.get("timeout_ms").is_some(),
            original.timeout_explicit
        );
        assert_eq!(decoded, original);
    }
}

/// 【翻译配置测试】【原生与原版】驼峰 JSON 与下划线 TOML 得到相同参数，原始服务字段保留，调试不输出凭据，无参数、无返回值
#[test]
fn native_and_legacy_translation_sections_preserve_private_vendor_fields() {
    let root = tempfile::tempdir().unwrap();
    let json = root.path().join("config.json");
    let toml = root.path().join("config.toml");
    std::fs::write(&json, r#"{"translation":{"targetLanguage":" English ","provider":"plugin:gemini","timeoutMs":1,"gemini":{"apiKey":"private-key"}}}"#).unwrap();
    std::fs::write(&toml, "[translation]\ntarget_language = ' English '\nprovider = 'plugin:gemini'\ntimeout_ms = 1\n[translation.gemini]\napiKey = 'private-key'\n").unwrap();
    let original = read_at(&json).unwrap();
    let native = read_at(&toml).unwrap();
    assert_eq!(original, native);
    assert_eq!(native.target_language, "English");
    assert_eq!(native.timeout_ms, 1000);
    assert_eq!(native.settings["gemini"]["apiKey"], "private-key");
    assert!(!format!("{native:?}").contains("private-key"));
}

/// 【翻译配置测试】【读取拒绝】链接、多重硬链接、超大文件和非法类型都不能当作可用配置，无参数、无返回值
#[test]
fn unsafe_config_files_and_invalid_types_are_rejected() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("config.toml");
    let link = root.path().join("link.toml");
    std::fs::write(&path, "[translation]\nprovider = 42\n").unwrap();
    assert!(read_at(&path).is_err());
    std::fs::write(&path, "[translation]\nprovider = 'auto'\n").unwrap();
    crate::utils::test_files::link_file(&path, &link).unwrap();
    assert!(read_at(&link).is_err());
    std::fs::remove_file(&link).unwrap();
    std::fs::hard_link(&path, &link).unwrap();
    assert!(read_at(&path).is_err());
    std::fs::remove_file(&link).unwrap();
    std::fs::write(&path, " ".repeat(1024 * 1024 + 1)).unwrap();
    assert!(read_at(&path).is_err());
}

/// 【翻译配置测试】【共享保存】历史与语言同时更新时互不覆盖，厂商凭据、未知字段和行尾注释保持原样，无参数、无返回值
#[test]
fn language_and_history_updates_use_the_same_atomic_lock() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("config.toml");
    std::fs::write(&path, "# private config\n[custom]\nvalue=17\n[history]\nlimit=50 # history policy\n[translation]\ntarget_language='Japanese' # language\napiKey='private-key'\n").unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let history_path = path.clone();
    let history_barrier = barrier.clone();
    let history = std::thread::spawn(move || {
        history_barrier.wait();
        for limit in 1..=8 {
            super::super::history::update_at(
                &history_path,
                super::super::history::Update::Limit(limit),
            )
            .unwrap();
        }
    });
    let language_path = path.clone();
    let language = std::thread::spawn(move || {
        barrier.wait();
        for _ in 0..8 {
            save_target_at(&language_path, "French").unwrap();
        }
    });
    history.join().unwrap();
    language.join().unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    let value: toml::Table = toml::from_str(&text).unwrap();
    assert_eq!(value["history"]["limit"].as_integer(), Some(8));
    assert_eq!(read_at(&path).unwrap().target_language, "French");
    assert_eq!(value["custom"]["value"].as_integer(), Some(17));
    assert!(text.contains("apiKey='private-key'"));
    assert!(text.contains("# language") && text.contains("# history policy"));
}

/// 【翻译配置测试】【保存失败】空目标、损坏配置和符号链接都不能修改原文件或创建新配置，无参数、无返回值
#[test]
fn failed_language_persistence_never_overwrites_the_previous_config() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("config.toml");
    assert!(save_target_at(&path, " ").is_err());
    assert!(!path.exists());
    std::fs::write(&path, "[broken\n").unwrap();
    assert!(save_target_at(&path, "English").is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "[broken\n");
    std::fs::remove_file(&path).unwrap();
    let target = root.path().join("target.toml");
    std::fs::write(&target, "# original\n").unwrap();
    crate::utils::test_files::link_file(&target, &path).unwrap();
    assert!(save_target_at(&path, "English").is_err());
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "# original\n");
}

/// 【翻译配置测试】【自由输入保留】英文配置中的中文自由语言名称保持原样，不擅自猜测原版配置的界面语言，无参数、无返回值
#[test]
fn localized_free_text_is_not_reinterpreted_while_reading_or_saving_config() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("config.toml");
    std::fs::write(
        &path,
        "[translation]\ntarget_language='简体中文' # manual input\n",
    )
    .unwrap();
    assert_eq!(read_at(&path).unwrap().target_language, "简体中文");
    assert_eq!(save_target_at(&path, " 繁体中文 ").unwrap(), "繁体中文");
    assert_eq!(read_at(&path).unwrap().target_language, "繁体中文");
    assert!(
        std::fs::read_to_string(&path)
            .unwrap()
            .contains("# manual input")
    );
}

/// 【翻译配置测试】【默认值隔离】缺少全局超时时保留厂商三十秒默认值，显式六十秒和厂商覆盖仍优先，无参数、无返回值
#[test]
fn absent_global_timeout_does_not_override_the_vendor_default() {
    use crate::translation::resolve::{self, Service};
    for (fields, expected) in [
        (serde_json::json!({}), 30000),
        (serde_json::json!({"timeoutMs":60000}), 60000),
        (serde_json::json!({"timeout_ms":1700}), 1700),
        (
            serde_json::json!({"timeoutMs":60000,"tencent":{"timeoutMs":2300}}),
            2300,
        ),
    ] {
        let mut fields = fields;
        fields["tencent"]["secretId"] = serde_json::json!("test-id");
        fields["tencent"]["secretKey"] = serde_json::json!("test-key");
        let config: Translation = serde_json::from_value(fields).unwrap();
        assert_eq!(
            config.timeout_ms,
            if expected == 1700 { 1700 } else { 60000 }
        );
        let Service::Tencent(service) =
            resolve::service(&config.root(), "plugin:tencent-tmt", &|_| None).unwrap()
        else {
            panic!("expected tencent");
        };
        assert_eq!(service.timeout.as_millis(), expected);
    }
    assert!(serde_json::from_value::<Translation>(serde_json::json!({"timeoutMs":null})).is_err());
}
