//! 【设置回归】【开机启动】验证原生开关事件、配置格式和默认继承，不注册测试机器的启动项

use super::super::{catalog::ENTRIES, window::Control, window::Preferences};

/// 【设置回归】【开关操作】无参数，通过真实开关事件启用、关闭及恢复默认，不修改托盘和快捷键配置
#[test]
fn login_startup_toggle_saves_and_resets_without_changing_desktop_controls() {
    let mut state = Preferences::new(false).unwrap();
    let index = ENTRIES
        .iter()
        .position(|entry| entry.path() == "desktop.autostart")
        .unwrap();
    assert_eq!(state.values["desktop"]["autostart"], false);
    let tray = state.values["desktop"]["tray_enabled"].clone();
    let shortcuts = state.values["desktop"]["shortcuts_enabled"].clone();
    state.activate(Control::Row(index));
    assert_eq!(state.values["desktop"]["autostart"], true);
    assert_eq!(state.values["desktop"]["tray_enabled"], tray);
    assert_eq!(state.values["desktop"]["shortcuts_enabled"], shortcuts);
    state.activate(Control::Row(index));
    assert_eq!(state.values["desktop"]["autostart"], false);
    state.activate(Control::Row(index));
    state.activate(Control::Reset(index));
    assert_eq!(state.values["desktop"]["autostart"], false);
    assert!(!crate::config::settings_store::explicit(
        &state.source,
        "desktop.autostart"
    ));
}

/// 【设置回归】【格式兼容】无参数，验证 TOML 与原版 JSON 保存启动开关时保留未知字段，拒绝字符串布尔值
#[test]
fn login_startup_roundtrips_both_configuration_formats() {
    let directory = tempfile::tempdir().unwrap();
    for (name, original) in [
        ("config.toml", "# keep\n[custom]\nvalue = 7\n"),
        ("config.json", r#"{"custom":{"value":7}}"#),
    ] {
        let path = directory.path().join(name);
        std::fs::write(&path, original).unwrap();
        crate::config::preferences::write(&path, "desktop", "autostart", true.into()).unwrap();
        assert!(
            crate::config::preferences::load(&path)
                .unwrap()
                .desktop
                .autostart
        );
        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(saved.contains("custom") && saved.contains('7'));
        assert!(
            crate::config::preferences::write(&path, "desktop", "autostart", "true".into())
                .is_err()
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), saved);
    }
}
