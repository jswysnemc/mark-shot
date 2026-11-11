//! 【桌面集成测试】【会话隔离】覆盖终端与桌面使用不同 XWayland 的录制控制场景

use super::session_identity;
use std::path::Path;

/// 【桌面集成测试】【私有环境】按 config、Wayland 和 X11 显示名计算标识，不修改进程环境
fn identity(config: &str, wayland: Option<&str>, display: &str) -> u64 {
    session_identity(Path::new(config), |key| {
        match key {
            "WAYLAND_DISPLAY" => wayland,
            "DISPLAY" => Some(display),
            "DBUS_SESSION_BUS_ADDRESS" => Some("private-bus"),
            _ => None,
        }
        .map(Into::into)
    })
    .unwrap()
}

/// 【桌面集成测试】【Wayland一致性】无参数，验证同配置的桌面窗口和终端共享控制目录
#[test]
fn wayland_control_ignores_xwayland_display() {
    assert_eq!(
        identity("config.toml", Some("wayland-2"), ":0"),
        identity("config.toml", Some("wayland-2"), ":1")
    );
}

/// 【桌面集成测试】【独立会话】无参数，验证不同配置、Wayland 和 X11 显示仍彼此隔离
#[test]
fn distinct_configs_and_displays_stay_isolated() {
    let current = identity("config.toml", Some("wayland-2"), ":0");
    assert_ne!(current, identity("other.toml", Some("wayland-2"), ":0"));
    assert_ne!(current, identity("config.toml", Some("wayland-3"), ":0"));
    assert_ne!(
        identity("config.toml", None, ":0"),
        identity("config.toml", None, ":1")
    );
}
