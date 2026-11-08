//! 【主题配置】【独立保存】共享原子锁，切换主题保留语言、历史、翻译及自定义颜色

use super::{Config, Theme};
use crate::theme::{Mode, toolbar::Palette};
use std::{io, path::Path};

impl Theme {
    /// 【主题配置】【原版默认】按 mode 返回共享角色配色，几何参数始终沿用默认值
    pub fn for_mode(mode: Mode) -> Self {
        let palette = Palette::new(mode);
        Self {
            background: palette.panel_fill,
            foreground: palette.text,
            muted: palette.muted_text,
            hover: palette.hover_fill,
            accent: palette.active_fill,
            on_accent: palette.text,
            border: palette.panel_border,
            field: palette.control_fill,
            track: palette.slider_groove,
            caret: palette.strong_text,
            ..Self::default()
        }
    }

    /// 【主题配置】【颜色更新】从 source 更新共享颜色，保留 self 的字体尺寸、间距与圆角，无返回值
    pub(crate) fn copy_colors(&mut self, source: &Self) {
        macro_rules! copy {
            ($($field:ident),* $(,)?) => { $(self.$field = source.$field;)* };
        }
        copy!(
            background,
            foreground,
            muted,
            hover,
            accent,
            on_accent,
            border,
            border_on_light,
            field,
            track,
            caret,
            text_selection
        );
    }
}

/// 【主题配置】【启动配色】按 text 的明确主题更新 config，未选择时使用浅色；内置颜色继承主题，自定义覆盖保持原样，无返回值
pub(super) fn apply(config: &mut Config, text: &str) {
    let Ok(root) = text.parse::<toml::Table>() else {
        return;
    };
    if let Ok(root) = serde_json::to_value(root) {
        apply_root(config, &root);
    }
}

/// 【主题配置】【共用角色】按 root 的明确角色颜色更新 config，JSON 与 TOML 沿用已解析主题，旧模板内置颜色继续继承，无返回值
pub(super) fn apply_root(config: &mut Config, root: &serde_json::Value) {
    let theme = root.get("theme").and_then(serde_json::Value::as_object);
    let mode = config.ui.theme;
    let defaults = Theme::for_mode(mode);
    config.theme_custom = 0;
    macro_rules! inherit {
        ($($field:ident),* $(,)?) => {
            let mut index = 0;
            $(
                if !theme.is_some_and(|table| table.contains_key(stringify!($field))) || is_builtin(stringify!($field), config.theme.$field) {
                    config.theme.$field = defaults.$field;
                } else if !is_builtin(stringify!($field), config.theme.$field) {
                    config.theme_custom |= 1 << index;
                }
                index += 1;
            )*
            let _ = index;
        };
    }
    inherit!(
        background,
        foreground,
        muted,
        hover,
        accent,
        on_accent,
        border,
        border_on_light,
        field,
        track,
        caret,
        text_selection
    );
}

/// 【主题配置】【替代配色】按 config 生成另一主题，保留显式自定义角色与全部几何，返回新配色
#[cfg(test)]
pub(crate) fn alternate(config: &Config) -> Theme {
    with_mode(config, config.ui.theme.opposite())
}

/// 【主题配置】【指定配色】按 config 和 mode 返回主题，保留自定义角色及几何，系统切换使用同一规则
pub(crate) fn with_mode(config: &Config, mode: Mode) -> Theme {
    let mut theme = config.theme.clone();
    theme.copy_colors(&Theme::for_mode(mode));
    macro_rules! restore {
        ($($field:ident),* $(,)?) => {
            let mut index = 0;
            $(
                if config.theme_custom & (1 << index) != 0 { theme.$field = config.theme.$field; }
                index += 1;
            )*
            let _ = index;
        };
    }
    restore!(
        background,
        foreground,
        muted,
        hover,
        accent,
        on_accent,
        border,
        border_on_light,
        field,
        track,
        caret,
        text_selection
    );
    theme
}

/// 【主题配置】【旧模板识别】按 name、color 判断是否为内置明暗或旧模板颜色，返回是否随主题继承
pub(super) fn is_builtin(name: &str, color: crate::theme::Rgba) -> bool {
    use crate::theme::Rgba;
    let light = Theme::for_mode(Mode::Light);
    let dark = Theme::for_mode(Mode::Dark);
    let (first, second, legacy) = match name {
        "background" => (light.background, dark.background, Rgba(15, 17, 23, 230)),
        "foreground" => (light.foreground, dark.foreground, Rgba(229, 231, 235, 255)),
        "muted" => (light.muted, dark.muted, Rgba(156, 163, 175, 255)),
        "hover" => (light.hover, dark.hover, Rgba(45, 212, 191, 255)),
        "accent" => (light.accent, dark.accent, Rgba(153, 246, 228, 78)),
        "on_accent" => (light.on_accent, dark.on_accent, Rgba(26, 24, 20, 255)),
        "border" => (light.border, dark.border, Rgba(255, 255, 255, 28)),
        "border_on_light" => (
            light.border_on_light,
            dark.border_on_light,
            Rgba(16, 22, 23, 55),
        ),
        "field" => (light.field, dark.field, Rgba(39, 45, 47, 255)),
        "track" => (light.track, dark.track, Rgba(59, 70, 71, 255)),
        "caret" => (light.caret, dark.caret, Rgba(245, 224, 220, 255)),
        "text_selection" => (
            light.text_selection,
            dark.text_selection,
            Rgba(137, 180, 250, 110),
        ),
        _ => return false,
    };
    color == first
        || color == second
        || color == legacy
        || super::legacy_theme::contains(name, color)
}

/// 【主题配置】【单项更新】按 path 原子保存 mode，保留未知配置、注释和其他业务字段，返回写入结果
pub fn save_at(path: &Path, mode: Mode) -> io::Result<()> {
    super::settings_store::write(path, &[("ui.theme".into(), mode.name().into())])
}

/// 【主题配置】【当前保存】在命令行指定配置或默认配置内保存 mode，返回结果；没有配置目录时返回错误
pub fn save(mode: Mode) -> io::Result<()> {
    let path = path().ok_or_else(|| io::Error::other("configuration directory is unavailable"))?;
    save_at(&path, mode)
}

/// 【主题配置】【监听路径】返回命令行明确指定或默认配置路径，无参数，不读取文件
pub(crate) fn path() -> Option<std::path::PathBuf> {
    super::loading::config_path()
}

/// 【主题配置】【通知读取】按 path 有界读取已保存主题，缺省或删除使用浅色；无效文件返回错误，不改变进程状态
pub(crate) fn read_mode(path: &Path) -> io::Result<Mode> {
    let text = match super::persistence::read(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Mode::Light),
        Err(error) => return Err(error),
    };
    let root = super::document::root(path, &text)?;
    Ok(root
        .get("ui")
        .and_then(|ui| ui.get("theme"))
        .and_then(serde_json::Value::as_str)
        .map(Mode::parse)
        .unwrap_or_default())
}

#[cfg(test)]
#[path = "tests/interface_theme.rs"]
mod tests;
