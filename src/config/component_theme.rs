//! 【主题配置】【组件继承】识别旧模板角色值，避免工具栏和各类弹层固定为旧版深色

use crate::theme::{Rgba, Role};

/// 【主题配置】【内置组件色】按 role 和 color 判断是否为已公布的默认角色值，返回是否应继承当前主题，不读取配置
pub(super) fn is_builtin(role: Role, color: Rgba) -> bool {
    let (name, legacy): (&str, &[Rgba]) = match role {
        Role::Background => ("background", &[Rgba(30, 33, 36, 250)]),
        Role::Foreground => ("foreground", &[Rgba(229, 237, 239, 255)]),
        Role::Muted => ("muted", &[Rgba(162, 177, 180, 255)]),
        Role::Hover => ("hover", &[Rgba(180, 190, 254, 255)]),
        Role::Accent => (
            "accent",
            &[Rgba(75, 191, 155, 255), Rgba(77, 190, 157, 255)],
        ),
        Role::OnAccent => ("on_accent", &[Rgba(30, 33, 36, 255)]),
    };
    super::interface_theme::is_builtin(name, color) || legacy.contains(&color)
}

#[cfg(test)]
#[path = "tests/component_theme.rs"]
mod tests;
