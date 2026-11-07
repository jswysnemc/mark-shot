//! 【主题配置】【历史角色】全局与组件共用已发布的旧默认色，避免同色在不同配置层级产生不同继承行为

use crate::theme::Rgba;

/// 【主题配置】【历史匹配】按name和color返回是否为旧版默认角色值，不读取或修改配置，未知角色不匹配
pub(super) fn contains(name: &str, color: Rgba) -> bool {
    let colors: &[Rgba] = match name {
        "background" => &[Rgba(30, 33, 36, 250)],
        "foreground" => &[Rgba(229, 237, 239, 255)],
        "muted" => &[Rgba(162, 177, 180, 255)],
        "hover" => &[Rgba(180, 190, 254, 255)],
        "accent" => &[Rgba(75, 191, 155, 255), Rgba(77, 190, 157, 255)],
        "on_accent" => &[Rgba(30, 33, 36, 255)],
        _ => &[],
    };
    colors.contains(&color)
}

#[cfg(test)]
#[path = "tests/legacy_theme.rs"]
mod tests;
