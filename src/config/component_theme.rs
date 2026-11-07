//! 【主题配置】【组件继承】识别旧模板角色值，避免工具栏和各类弹层固定为旧版深色

use crate::theme::{Rgba, Role};

/// 【主题配置】【内置组件色】按 role 和 color 判断是否为已公布的默认角色值，返回是否应继承当前主题，不读取配置
pub(super) fn is_builtin(role: Role, color: Rgba) -> bool {
    let name = match role {
        Role::Background => "background",
        Role::Foreground => "foreground",
        Role::Muted => "muted",
        Role::Hover => "hover",
        Role::Accent => "accent",
        Role::OnAccent => "on_accent",
    };
    super::interface_theme::is_builtin(name, color)
}

#[cfg(test)]
#[path = "tests/component_theme.rs"]
mod tests;
