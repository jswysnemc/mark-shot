//! 【应用设置】【启动覆盖】命令行只覆盖当前进程，重新读取偏好时继续保留启动约束

use super::Config;
use std::sync::OnceLock;

#[derive(Default)]
struct Overrides {
    notifications: Option<bool>,
    level: Option<Option<String>>,
    path: Option<Option<String>>,
    overlay: Option<crate::backend::OverlayShell>,
}
static OVERRIDES: OnceLock<Overrides> = OnceLock::new();

/// 【应用设置】【覆盖记录】记录显式通知、日志和窗口参数，空值代表未指定；无返回值且不写文件
pub(crate) fn record(
    notifications: Option<bool>,
    level: Option<String>,
    path: Option<String>,
    overlay: Option<crate::backend::OverlayShell>,
) {
    let _ = OVERRIDES.set(Overrides {
        notifications,
        level: level.map(Some),
        path: path.map(Some),
        overlay,
    });
}

/// 【应用设置】【覆盖应用】将已记录的启动约束应用到 config，返回空值，不覆盖其他用户偏好
pub(super) fn apply(config: &mut Config) {
    if let Some(overrides) = OVERRIDES.get() {
        overrides.apply(config);
    }
}

impl Overrides {
    /// 【应用设置】【约束合并】按 config 应用显式参数，即使启动时与文件值相同也继续保留覆盖
    fn apply(&self, config: &mut Config) {
        let overrides = self;
        if let Some(enabled) = overrides.notifications {
            config.notifications.enabled = enabled;
        }
        if let Some(level) = &overrides.level {
            config.log.level = level.clone();
        }
        if let Some(path) = &overrides.path {
            config.log.path = path.clone();
        }
        if let Some(overlay) = overrides.overlay {
            config.general.overlay = overlay;
        }
    }
}

/// 【应用设置】【命令行来源】按 path 返回该字段是否受显式启动参数覆盖，不读取配置文件
pub(crate) fn contains(path: &str) -> bool {
    OVERRIDES.get().is_some_and(|overrides| match path {
        "notifications.enabled" => overrides.notifications.is_some(),
        "log.level" => overrides.level.is_some(),
        "log.path" => overrides.path.is_some(),
        "general.overlay" => overrides.overlay.is_some(),
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    /// 【应用设置】【显式参数回归】无参数，关闭通知在配置重载前后保持关闭，不修改其他偏好
    #[test]
    fn explicit_no_notify_survives_reload_even_when_initially_false() {
        let overrides = Overrides {
            notifications: Some(false),
            ..Default::default()
        };
        let mut config = Config::default();
        config.notifications.enabled = false;
        overrides.apply(&mut config);
        config.notifications.enabled = true;
        config.save.directory = "changed".into();
        overrides.apply(&mut config);
        assert!(!config.notifications.enabled);
        assert_eq!(config.save.directory, "changed");
    }
}
