//! 【截图配置测试】【兼容与优先级】验证 Qt 设置迁移、原文写入和命令行范围覆盖

use crate::config::{capture::*, preferences, settings_store};

/// 【截图配置测试】【旧字段往返】无参数，保留旧分组及未知字段，并实际更新每项行为与工具栏尺寸
#[test]
fn qt_capture_and_toolbar_fields_round_trip_without_losing_other_settings() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    std::fs::write(&path, r#"{"screenshot":{"includeCursor":true,"hideOwnWindows":false,"doubleClickAction":"pin","middleClickAction":"save-as","freezeScope":"cursor-screen","toolbarPosition":"right","wayland":{"kde":{"kwinScreenshot":{"enabled":false}}}},"toolbar":{"iconSize":36,"fontSize":18},"unrelated":123}"#).unwrap();
    let config = preferences::load(&path).unwrap();
    assert_eq!(config.capture.double_click_action, ClickAction::Pin);
    assert_eq!(config.capture.middle_click_action, ClickAction::SaveAs);
    assert_eq!(config.capture.freeze_scope, FreezeScope::CursorScreen);
    assert_eq!(config.capture.toolbar_position, ToolbarPosition::Right);
    assert!(!config.capture.hide_own_windows && !config.capture.kde_kwin_screenshot);
    assert_eq!(
        (config.toolbar.icon_size, config.toolbar.font_size),
        (36, 18)
    );
    settings_store::write(
        &path,
        &[
            ("capture.double_click_action".into(), "cancel".into()),
            ("capture.kde_kwin_screenshot".into(), true.into()),
            ("toolbar.icon_size".into(), 24.into()),
            ("capture.toolbar_position".into(), "left".into()),
        ],
    )
    .unwrap();
    let updated = preferences::load(&path).unwrap();
    assert_eq!(updated.capture.double_click_action, ClickAction::Cancel);
    assert!(updated.capture.kde_kwin_screenshot && updated.startup.include_cursor);
    assert_eq!(updated.toolbar.icon_size, 24);
    assert_eq!(updated.capture.toolbar_position, ToolbarPosition::Left);
    let root = settings_store::source(&path).unwrap();
    assert_eq!(root["unrelated"], 123);
    assert!(root.get("capture").is_none());
    assert_eq!(root["screenshot"]["doubleClickAction"], "cancel");
    assert_eq!(root["toolbar"]["iconSize"], 24);
    assert_eq!(root["screenshot"]["toolbarPosition"], "left");
}

/// 【截图配置测试】【原生配置】无参数，TOML 保存恢复完整策略，非法尺寸在设置入口拒绝
#[test]
fn native_capture_values_and_toolbar_validation_are_consistent() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.toml");
    settings_store::write(
        &path,
        &[
            ("capture.middle_click_action".into(), "copy".into()),
            ("capture.toolbar_position".into(), "left".into()),
            ("toolbar.font_size".into(), 24.into()),
        ],
    )
    .unwrap();
    let config = preferences::load(&path).unwrap();
    assert_eq!(config.capture.middle_click_action, ClickAction::Copy);
    assert_eq!(config.capture.toolbar_position, ToolbarPosition::Left);
    assert_eq!(config.toolbar.font_size, 24);
    assert!(preferences::write(&path, "toolbar", "icon_size", 49.into()).is_err());
    assert!(preferences::write(&path, "toolbar", "font_size", 7.into()).is_err());
}

/// 【截图配置测试】【范围覆盖】无参数，明确单屏和全部输出优先，普通配置不缩减全屏标注范围
#[test]
fn explicit_output_selection_overrides_default_freeze_scope() {
    let config = Capture {
        freeze_scope: FreezeScope::CursorScreen,
        ..Default::default()
    };
    assert!(config.one_monitor(false, false, false));
    assert!(!config.one_monitor(false, true, false));
    assert!(!config.one_monitor(false, false, true));
    assert!(config.one_monitor(true, false, true));
    assert!(!Capture::default().one_monitor(false, false, false));
}

/// 【截图配置测试】【旧别名恢复】无参数，旧路径保存后可恢复默认，不残留生效的旧值
#[test]
fn legacy_aliases_save_and_reset_without_restoring_old_values() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    std::fs::write(
        &path,
        r#"{"freezeDisplayScope":"cursor-screen","screenCapture":{"hideOwnWindowsDuringCapture":false},"toolbar":{"iconSize":36,"fontSize":18},"custom":17}"#,
    )
    .unwrap();
    let fields = [
        ("capture.freeze_scope".into(), "all-screens".into()),
        ("capture.hide_own_windows".into(), true.into()),
        ("toolbar.icon_size".into(), 24.into()),
        ("toolbar.font_size".into(), 16.into()),
    ];
    settings_store::write(&path, &fields).unwrap();
    let saved = preferences::load(&path).unwrap();
    assert_eq!(saved.capture.freeze_scope, FreezeScope::AllScreens);
    assert!(saved.capture.hide_own_windows);
    assert_eq!((saved.toolbar.icon_size, saved.toolbar.font_size), (24, 16));
    settings_store::write(
        &path,
        &fields
            .into_iter()
            .map(|(key, _)| (key, serde_json::Value::Null))
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let reset = preferences::load(&path).unwrap();
    assert_eq!(reset.capture.freeze_scope, FreezeScope::AllScreens);
    assert!(reset.capture.hide_own_windows);
    assert_eq!((reset.toolbar.icon_size, reset.toolbar.font_size), (22, 11));
    let root = settings_store::source(&path).unwrap();
    assert!(root.get("freezeDisplayScope").is_none());
    assert_eq!(root["custom"], 17);
}
