//! 【主题配置测试】【组件矩阵】旧模板颜色继承当前主题，自定义颜色与几何始终保留

use crate::{
    config::{Theme, loading},
    theme::{Mode, Rgba},
};

/// 【主题配置测试】【遗留组件】两种主题下检查工具栏、属性、颜色、模型、放大镜、提示和识别状态，确保旧颜色不阻止继承
#[test]
fn legacy_component_defaults_follow_both_themes_on_startup() {
    for mode in [Mode::Light, Mode::Dark] {
        let source = format!(
            "[ui]\ntheme=\"{}\"\n[toolbar]\nbackground=\"#1E2124FA\"\nicon=\"#E5EDEF\"\nicon_hovered=\"#1E2124\"\nbutton_hovered=\"#B4BEFE\"\n[settings_panel]\nbackground=\"#1E2124FA\"\ntext=\"#E5EDEF\"\n[color_picker]\nbackground=\"#1E2124FA\"\nlabel=\"#A2B1B4\"\n[model_picker]\nbackground=\"#1E2124FA\"\n[magnifier]\nlabel_background=\"#1E2124FA\"\nlabel_text=\"#E5EDEF\"\n[toasts]\nbackground=\"#1E2124FA\"\ntext=\"#E5EDEF\"\n[ocr.look]\nbadge_background=\"#1E2124FA\"\nbadge_icon=\"#E5EDEF\"\n",
            mode.name()
        );
        let config = loading::parse(&source, "component-theme-test");
        let expected = Theme::for_mode(mode);
        for background in [
            config.toolbar.background,
            config.settings_panel.background,
            config.color_picker.background,
            config.model_picker.background,
            config.magnifier.label_background,
            config.toasts.background,
            config.ocr.look.badge_background,
        ] {
            assert_eq!(background.get(), expected.background);
        }
        for foreground in [
            config.toolbar.icon,
            config.settings_panel.text,
            config.magnifier.label_text,
            config.toasts.text,
            config.ocr.look.badge_icon,
        ] {
            assert_eq!(foreground.get(), expected.foreground);
        }
        assert_eq!(config.toolbar.icon_hovered.get(), expected.on_accent);
        assert_eq!(config.toolbar.button_hovered.get(), expected.hover);
        assert_eq!(config.color_picker.label.get(), expected.muted);
    }
}

/// 【主题配置测试】【自定义保留】修改过的组件颜色和尺寸不会因主题修正而改变，图像与绘图工具色不属于组件继承
#[test]
fn customized_component_colors_and_geometry_are_retained() {
    let config = loading::parse(
        "[ui]\ntheme=\"light\"\n[toolbar]\nbackground=\"#112233\"\nbutton_size=35.0\n[model_picker]\nbackground=\"#123456\"\n[tools]\ncolor=\"#1E2124FA\"\n",
        "component-theme-test",
    );
    assert_eq!(config.toolbar.background.get(), Rgba(17, 34, 51, 255));
    assert_eq!(config.toolbar.button_size, 35.0);
    assert_eq!(config.model_picker.background.get(), Rgba(18, 52, 86, 255));
    assert_eq!(config.tools.color, Rgba(30, 33, 36, 250));
}
