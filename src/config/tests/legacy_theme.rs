//! 【主题兼容测试】【层级一致】旧全局与组件配色使用同一继承规则，正文与自定义颜色保持不变

use crate::{
    config::{Theme, interface_theme, loading},
    theme::{Mode, Rgba},
};

const ROLES: &[(&str, &str)] = &[
    ("background", "#1E2124FA"),
    ("foreground", "#E5EDEF"),
    ("muted", "#A2B1B4"),
    ("hover", "#B4BEFE"),
    ("accent", "#4BBF9B"),
    ("on_accent", "#1E2124"),
];

/// 【主题兼容测试】【配置构造】按mode与json生成含旧全局色及标注色的配置文本，返回字符串，不读写文件
fn source(mode: Mode, json: bool) -> String {
    if json {
        let theme = ROLES
            .iter()
            .map(|(name, value)| ((*name).to_owned(), serde_json::Value::from(*value)))
            .collect::<serde_json::Map<_, _>>();
        serde_json::json!({"ui":{"theme":mode.name()},"theme":theme,"tools":{"color":"#1E2124FA"}})
            .to_string()
    } else {
        format!(
            "[ui]\ntheme=\"{}\"\n[tools]\ncolor=\"#1E2124FA\"\n[theme]\n{}",
            mode.name(),
            ROLES
                .iter()
                .map(|(name, value)| format!("{name}=\"{value}\"\n"))
                .collect::<String>()
        )
    }
}

/// 【主题兼容测试】【启动与切换】无参数，TOML和JSON下旧全局角色跟随主题，标注色不参与迁移，无返回值
#[test]
fn global_legacy_roles_follow_both_modes_without_changing_annotation_color() {
    for json in [false, true] {
        for mode in [Mode::Light, Mode::Dark] {
            let config = loading::parse(
                &source(mode, json),
                if json { "test.json" } else { "test.toml" },
            );
            let expected = Theme::for_mode(mode);
            assert_eq!(config.theme.background, expected.background);
            assert_eq!(config.theme.foreground, expected.foreground);
            assert_eq!(config.theme.muted, expected.muted);
            assert_eq!(config.theme.hover, expected.hover);
            assert_eq!(config.theme.accent, expected.accent);
            assert_eq!(config.theme.on_accent, expected.on_accent);
            assert_eq!(config.toolbar.background.get(), expected.background);
            assert_eq!(config.tools.color, Rgba(30, 33, 36, 250));
            assert_eq!(
                interface_theme::alternate(&config).background,
                Theme::for_mode(mode.opposite()).background
            );
        }
    }
}

/// 【主题兼容测试】【保存重开】无参数，两种格式往返保存后继续继承主题，保留接近旧色但不同的显式自定义颜色，无返回值
#[test]
fn saving_legacy_roles_preserves_custom_colors_and_other_sections() {
    for json in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory
            .path()
            .join(if json { "config.json" } else { "config.toml" });
        std::fs::write(
            &path,
            source(Mode::Dark, json).replace("#A2B1B4", "#A2B1B5"),
        )
        .unwrap();
        for mode in [Mode::Light, Mode::Dark, Mode::Light] {
            interface_theme::save_at(&path, mode).unwrap();
            let text = std::fs::read_to_string(&path).unwrap();
            let config = loading::parse(&text, path.to_str().unwrap());
            assert_eq!(config.theme.background, Theme::for_mode(mode).background);
            assert_eq!(config.theme.muted, Rgba(162, 177, 181, 255));
            assert_eq!(config.tools.color, Rgba(30, 33, 36, 250));
        }
    }
}
