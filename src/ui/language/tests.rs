use super::*;

/// 【界面语言测试】【模式别名】核对原版配置别名，环境专用区域值不能擅自扩充配置模式，无参数、无返回值
#[test]
fn configured_modes_accept_only_the_original_aliases() {
    for value in ["english", "EN", "en_us", " en-US "] {
        assert_eq!(Mode::parse(value), Mode::English);
    }
    for value in ["chinese", "ZH", "zh_cn", " zh-CN ", "cn"] {
        assert_eq!(Mode::parse(value), Mode::Chinese);
    }
    for value in ["system", "", "unknown", "zh-TW", "zh_HK", "japanese"] {
        assert_eq!(Mode::parse(value), Mode::System);
    }
}

/// 【界面语言测试】【解析顺序】非空环境覆盖优先，其次配置，最后识别系统中文区域，无参数、无返回值
#[test]
fn override_configuration_and_system_locale_match_the_original_order() {
    for value in ["zh_TW.UTF-8", "zh-HK", "Chinese", " cn "] {
        assert_eq!(
            Language::resolve(Some(value), Mode::English, Some("en_US")),
            Language::Chinese
        );
    }
    assert_eq!(
        Language::resolve(Some("invalid"), Mode::Chinese, Some("zh_CN")),
        Language::English
    );
    assert_eq!(
        Language::resolve(Some(" "), Mode::Chinese, Some("en_US")),
        Language::Chinese
    );
    assert_eq!(
        Language::resolve(None, Mode::English, Some("zh_CN")),
        Language::English
    );
    for locale in ["zh", "zh_CN.UTF-8", "zh-TW", "zh_HK"] {
        assert_eq!(
            Language::resolve(None, Mode::System, Some(locale)),
            Language::Chinese
        );
    }
    for locale in ["en_US", "ja_JP", "zhunknown", "C"] {
        assert_eq!(
            Language::resolve(None, Mode::System, Some(locale)),
            Language::English
        );
    }
}

/// 【界面语言测试】【参数隔离】参数中的编号占位符及任意正文不再次翻译或替换，未知键保持原文，无参数、无返回值
#[test]
fn interpolation_preserves_business_values_without_recursive_replacement() {
    assert_eq!(Language::Chinese.text("Source image"), "原图");
    assert_eq!(Language::English.text("Source image"), "Source image");
    assert_eq!(
        Language::Chinese.text("user supplied text"),
        "user supplied text"
    );
    assert_eq!(
        Language::Chinese.text("Always on Top: On"),
        "Always on Top: On"
    );
    assert_eq!(
        Language::Chinese.format("Translation failed (%1): %2", &["Copy %2", "raw %1"]),
        "翻译失败（Copy %2）：raw %1"
    );
    assert_eq!(
        Language::English.format("%1 rows × %2 columns", &["2", "4"]),
        "2 rows × 4 columns"
    );
    assert_eq!(
        Language::Chinese.format("%1 rows × %2 columns", &["2", "4"]),
        "2 行 × 4 列"
    );
    assert_eq!(
        Language::Chinese.format("%3 / %% / %0", &["raw"]),
        "%3 / %% / %0"
    );
}

/// 【界面语言测试】【原版文案】核对完整原版采集的两种语言各 63 个键，预设语言展示表必须与界面文案一致，无参数、无返回值
#[test]
fn catalog_matches_every_actual_qt_translation_and_language_choice() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/qt-ui-language.json")).unwrap();
    for entry in fixture["languages"].as_array().unwrap() {
        let language = if entry["name"] == "zh" {
            Language::Chinese
        } else {
            Language::English
        };
        let catalog = entry["catalog"].as_object().unwrap();
        assert_eq!(catalog.len(), 63);
        for (source, expected) in catalog {
            assert_eq!(
                language.text(source),
                expected.as_str().unwrap(),
                "{source}"
            );
        }
        for (value, label) in crate::translation::language::OPTIONS
            .iter()
            .zip(crate::translation::language::labels(language))
        {
            assert_eq!(language.text(value), *label);
        }
    }
}
