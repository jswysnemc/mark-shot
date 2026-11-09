//! 【桌面设置】【字段目录】公开托盘开关和全局截图绑定，区分内部编辑快捷键

use super::{Entry, Kind, Page};

pub(super) const ENTRIES: &[Entry] = &[
    Entry {
        page: Page::Appearance,
        section: "desktop",
        key: "tray_enabled",
        labels: ("显示系统托盘图标", "Show system tray icon"),
        kind: Kind::Toggle,
    },
    Entry {
        page: Page::Keys,
        section: "desktop",
        key: "shortcuts_enabled",
        labels: ("启用全局截图快捷键", "Enable global screenshot shortcut"),
        kind: Kind::Toggle,
    },
    Entry {
        page: Page::Keys,
        section: "desktop",
        key: "screenshot_shortcut",
        labels: ("全局截图快捷键", "Global screenshot shortcut"),
        kind: Kind::Shortcut,
    },
];
