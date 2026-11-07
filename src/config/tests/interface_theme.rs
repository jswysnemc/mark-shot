//! 【主题配置测试】【默认与保存】核对浅色默认、旧配色兼容及跨业务原子更新

use super::*;

/// 【主题配置测试】【默认浅色】空配置和模板都使用原版浅色角色；暗色配置只替换颜色，尺寸保持一致，无参数、无返回值
#[test]
fn light_is_the_default_and_dark_changes_only_colors() {
    let light = super::super::loading::parse("", "test");
    let dark = super::super::loading::parse("[ui]\ntheme = \"dark\"\n", "test");
    assert_eq!(light.ui.theme, Mode::Light);
    assert_eq!(light.theme, Theme::for_mode(Mode::Light));
    assert_eq!(dark.ui.theme, Mode::Dark);
    assert_eq!(dark.theme, Theme::for_mode(Mode::Dark));
    assert_eq!(dark.toolbar.background.get(), dark.theme.background);
    assert_eq!(light.theme.panel_height, dark.theme.panel_height);
    assert_eq!(light.theme.font_size, dark.theme.font_size);
}

/// 【主题配置测试】【覆盖优先】旧模板内置颜色使用默认浅色，显式自定义颜色与尺寸继续生效，未知名称回退浅色，无参数、无返回值
#[test]
fn legacy_background_and_explicit_colors_remain_compatible() {
    let read = |text| super::super::loading::parse(text, "test");
    let legacy = read("[theme]\nbackground = \"#0F1117E6\"\n");
    assert_eq!(legacy.ui.theme, Mode::Light);
    assert_eq!(
        legacy.theme.foreground,
        Theme::for_mode(Mode::Light).foreground
    );
    let explicit = read(
        "[ui]\ntheme = \"light\"\n[theme]\nbackground = \"#112233\"\nfont_size = 14.0\n[toolbar]\nicon = \"#AB1234\"\n",
    );
    assert_eq!(explicit.ui.theme, Mode::Light);
    assert_eq!(
        explicit.theme.background,
        crate::theme::Rgba(17, 34, 51, 255)
    );
    assert_eq!(explicit.theme.font_size, 14.0);
    assert_eq!(
        explicit.toolbar.icon.get(),
        crate::theme::Rgba(171, 18, 52, 255)
    );
    assert_eq!(read("[ui]\ntheme = \"missing\"\n").ui.theme, Mode::Light);
}

/// 【主题配置测试】【保留其他业务】两次保存只更新 ui.theme，注释、语言、历史和未知扩展配置保持原样，无参数、无返回值
#[test]
fn saving_a_theme_preserves_comments_language_and_other_sections() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "# retained\n[ui]\nlanguage = \"chinese\"\n[history]\nlimit = 23\n[custom]\nvalue = \"kept\"\n").unwrap();
    for mode in [Mode::Dark, Mode::Light] {
        save_at(&path, mode).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("# retained\n"));
        let document: toml::Value = toml::from_str(&text).unwrap();
        assert_eq!(document["ui"]["theme"].as_str(), Some(mode.name()));
        assert_eq!(document["ui"]["language"].as_str(), Some("chinese"));
        assert_eq!(document["history"]["limit"].as_integer(), Some(23));
        assert_eq!(document["custom"]["value"].as_str(), Some("kept"));
    }
}

/// 【主题配置测试】【错误不写入】非法配置结构不产生部分修改，缺失文件生成有效默认模板，无参数、无返回值
#[test]
fn invalid_config_is_preserved_and_missing_config_is_created() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "ui = 7\n").unwrap();
    assert!(save_at(&path, Mode::Dark).is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "ui = 7\n");
    std::fs::remove_file(&path).unwrap();
    save_at(&path, Mode::Dark).unwrap();
    let config = super::super::loading::parse(&std::fs::read_to_string(&path).unwrap(), "test");
    assert_eq!(config.ui.theme, Mode::Dark);
    assert_eq!(config.theme.background, Palette::new(Mode::Dark).panel_fill);
}

/// 【主题配置测试】【旧模板重开】旧模板内置颜色在保存时恢复继承，随后重开使用浅色；真正自定义颜色仍保留，无参数、无返回值
#[test]
fn switching_an_old_template_removes_builtin_overrides_before_reopening() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "[theme]\nbackground = \"#0F1117E6\"\nforeground = \"#E5E7EB\"\non_accent = \"#1A1814\"\nfield = \"#272D2F\"\ntrack = \"#3B4647\"\ncaret = \"#F5E0DC\"\nhover = \"#123456\"\n").unwrap();
    save_at(&path, Mode::Light).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    let reopened = super::super::loading::parse(&text, "test");
    assert_eq!(reopened.ui.theme, Mode::Light);
    let defaults = Theme::for_mode(Mode::Light);
    assert_eq!(reopened.theme.background, defaults.background);
    assert_eq!(reopened.theme.foreground, defaults.foreground);
    assert_eq!(reopened.theme.field, defaults.field);
    assert_eq!(reopened.theme.track, defaults.track);
    assert_eq!(reopened.theme.caret, defaults.caret);
    assert_eq!(reopened.theme.hover, crate::theme::Rgba(18, 52, 86, 255));
}

/// 【主题配置测试】【替代主题覆盖】切换时真正自定义的颜色和字体尺寸保留，其他颜色使用另一套原版令牌，无参数、无返回值
#[test]
fn alternate_theme_preserves_custom_colors_and_geometry() {
    let config = super::super::loading::parse(
        "[theme]\nforeground = \"#AB1234\"\nfont_size = 14.0\n",
        "test",
    );
    let alternate = alternate(&config);
    assert_eq!(alternate.foreground, config.theme.foreground);
    assert_eq!(alternate.font_size, 14.0);
    assert_eq!(alternate.background, Theme::for_mode(Mode::Dark).background);
}

/// 【主题配置测试】【启动残留】显式浅色必须覆盖旧模板内置深色角色，所有共享组件启动时直接得到浅色，无参数、无返回值
#[test]
fn explicit_light_replaces_builtin_dark_roles_before_the_first_frame() {
    let config = super::super::loading::parse(
        "[ui]\ntheme=\"light\"\n[theme]\nbackground=\"#0F1117E6\"\nforeground=\"#E5E7EB\"\nfield=\"#272D2F\"\ntrack=\"#3B4647\"\ncaret=\"#F5E0DC\"\n",
        "test",
    );
    let expected = Theme::for_mode(Mode::Light);
    assert_eq!(config.theme.background, expected.background);
    assert_eq!(config.theme.foreground, expected.foreground);
    assert_eq!(config.theme.field, expected.field);
    assert_eq!(config.theme.track, expected.track);
    assert_eq!(config.theme.caret, expected.caret);
    assert_eq!(config.toolbar.background.get(), expected.background);
    assert_eq!(config.settings_panel.background.get(), expected.background);
    assert_eq!(config.model_picker.background.get(), expected.background);
    assert_eq!(config.toasts.background.get(), expected.background);
}
