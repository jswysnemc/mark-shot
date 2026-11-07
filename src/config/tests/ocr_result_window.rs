//! 【OCR 窗口配置测试】【保存边界】核对独立默认、兼容键、原子保存和未知字段，不访问用户配置

use super::*;

/// 【OCR 窗口配置测试】【独立默认】贴图置顶不能影响结果窗口默认，原版 JSON 和原生 TOML 显式开关均可读取，无参数、无返回值
#[test]
fn result_stacking_is_independent_from_pinned_images_and_accepts_legacy_keys() {
    let root = tempfile::tempdir().unwrap();
    for (name, text, expected) in [
        ("config.toml", "[pinned]\nalways_on_top = true\n", false),
        (
            "config.json",
            r#"{"pinnedWindow":{"alwaysOnTop":true}}"#,
            false,
        ),
        (
            "config.toml",
            "[ocr_result_window]\nalways_on_top = true\n",
            true,
        ),
        (
            "config.json",
            r#"{"ocrResultWindow":{"alwaysOnTop":true}}"#,
            true,
        ),
    ] {
        let path = root.path().join(name);
        std::fs::write(&path, text).unwrap();
        assert_eq!(read_at(&path).unwrap().always_on_top, expected);
    }
}

/// 【OCR 窗口配置测试】【定向更新】保存保持贴图、语言、未知参数和行尾注释，既有兼容键不复制成新键，无参数、无返回值
#[test]
fn stacking_save_preserves_other_sections_and_comments() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("config.toml");
    std::fs::write(&path, "[pinned]\nalways_on_top = true\n[translation]\ntarget_language = 'English'\n[ocrResultWindow]\nalwaysOnTop = false # keep\nother = 4\n").unwrap();
    save_at(&path, true).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("alwaysOnTop = true # keep"));
    assert!(!text.contains("ocr_result_window"));
    assert!(text.contains("always_on_top = true"));
    assert!(text.contains("target_language = 'English'"));
    assert!(text.contains("other = 4"));
    assert!(read_at(&path).unwrap().always_on_top);
}

/// 【OCR 窗口配置测试】【失败保持】非法字段及链接均不能写入，保存失败原内容保持不变，无参数、无返回值
#[test]
fn failed_or_unsafe_save_keeps_the_original_file() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("config.toml");
    for text in [
        "ocr_result_window = 42\n",
        "[ocr_result_window]\nalways_on_top = 'bad'\n",
    ] {
        std::fs::write(&path, text).unwrap();
        assert!(read_at(&path).is_err());
        assert!(save_at(&path, true).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
    }
    std::fs::write(&path, "[ocr_result_window]\nalways_on_top = false\n").unwrap();
    let link = root.path().join("link.toml");
    crate::utils::test_files::link_file(&path, &link).unwrap();
    assert!(save_at(&link, true).is_err());
    assert!(!read_at(&path).unwrap().always_on_top);
}

/// 【OCR 窗口配置测试】【原版保存】原版 JSON 不转换成 TOML，未知厂商参数、贴图开关和目标语言保持原样，无参数、无返回值
#[test]
fn legacy_json_stacking_updates_preserve_unrelated_values() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("config.json");
    let before = serde_json::json!({"pinnedWindow":{"alwaysOnTop":true},"translation":{"targetLanguage":"English","custom":{"token":"local-only"}},"unknown":[1,2,3]});
    std::fs::write(&path, serde_json::to_vec(&before).unwrap()).unwrap();
    save_at(&path, true).unwrap();
    let after: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    for key in ["pinnedWindow", "translation", "unknown"] {
        assert_eq!(after[key], before[key]);
    }
    assert_eq!(after["ocrResultWindow"]["alwaysOnTop"], true);
    assert!(read_at(&path).unwrap().always_on_top);
    save_at(&path, false).unwrap();
    assert!(!read_at(&path).unwrap().always_on_top);
}

/// 【OCR 窗口配置测试】【原版新文件】新 JSON 路径首次保存创建合法对象，不写入 TOML 模板，无参数、无返回值
#[test]
fn a_missing_json_config_can_be_initialized_by_the_stacking_option() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("config.json");
    save_at(&path, true).unwrap();
    assert!(read_at(&path).unwrap().always_on_top);
    let value: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(value["ocrResultWindow"]["alwaysOnTop"], true);
}
