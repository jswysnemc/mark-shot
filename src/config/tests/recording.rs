//! 【录制配置测试】【面板偏好】验证旧配置默认隐藏、显示开关保存和设置说明

use crate::config::{
    Config,
    recording::Recording,
    settings::{ENTRIES, Kind, Page},
    settings_store,
};

/// 【录制配置测试】【旧配置兼容】旧配置缺少开关时保持后台录制行为，不影响已保存参数
#[test]
fn older_recording_options_keep_controls_hidden() {
    let config: Config = toml::from_str("[recording]\nfps=24\ncursor=false\n").unwrap();
    assert!(!config.recording.keep_controls_visible);
    assert_eq!(config.recording.fps, 24);
    assert!(!config.recording.cursor);
    let serialized = serde_json::to_value(Recording {
        keep_controls_visible: true,
        ..Default::default()
    })
    .unwrap();
    let decoded: Recording = serde_json::from_value(serialized).unwrap();
    assert!(decoded.keep_controls_visible);
    decoded.validate().unwrap();
}

/// 【录制配置测试】【设置保存】通过实际写入端保存和关闭开关，TOML 与 JSON 的其他录制选项保持原值
#[test]
fn visibility_setting_round_trips_without_replacing_other_options() {
    let directory = tempfile::tempdir().unwrap();
    for (name, content) in [
        ("config.toml", "[recording]\nfps=24\nfuture_option=true\n"),
        (
            "config.json",
            r#"{"recording":{"fps":24,"futureOption":true}}"#,
        ),
    ] {
        let path = directory.path().join(name);
        std::fs::write(&path, content).unwrap();
        for visible in [true, false] {
            settings_store::write(
                &path,
                &[("recording.keep_controls_visible".into(), visible.into())],
            )
            .unwrap();
            let values = settings_store::source(&path).unwrap();
            let text = std::fs::read_to_string(&path).unwrap();
            let config = crate::config::loading::parse(&text, name);
            // 1. 【录制配置测试】【实际读取】通过启动解析器确认保存值真正生效，不只检查原始文档
            assert_eq!(values["recording"]["keep_controls_visible"], visible);
            assert_eq!(config.recording.keep_controls_visible, visible);
            assert_eq!(config.recording.fps, 24);
            let future = if name.ends_with("json") {
                "futureOption"
            } else {
                "future_option"
            };
            assert_eq!(values["recording"][future], true);
        }
    }
}

/// 【录制配置测试】【设置注册】面板开关位于录制页，双语说明均提供截图与托盘入口及下次录制生效提示
#[test]
fn visibility_setting_has_explicit_control_entries_and_effect() {
    let entry = ENTRIES
        .iter()
        .find(|entry| entry.path() == "recording.keep_controls_visible")
        .unwrap();
    assert_eq!(entry.page, Page::Recording);
    assert!(matches!(entry.kind, Kind::Toggle));
    assert_eq!(entry.default_value(), false);
    entry.validate(&true.into()).unwrap();
    entry.validate(&false.into()).unwrap();
    assert!(entry.validate(&"true".into()).is_err());
    assert!(entry.help(true).contains("截图界面") && entry.help(true).contains("托盘"));
    assert!(entry.help(false).contains("capture") && entry.help(false).contains("tray"));
    assert!(entry.effect(true).contains("下次录制"));
    assert!(entry.effect(false).contains("next recording"));
}
