//! 【设置实际值回归】【默认解析】覆盖字段显示、工具优先级及目录解析，不改写配置

use super::*;
use serde_json::json;

/// 【设置实际值回归】【字段定位】按 path 返回共用设置目录中的字段，缺少字段时测试失败
fn entry(path: &str) -> &'static Entry {
    ENTRIES.iter().find(|entry| entry.path() == path).unwrap()
}

/// 【设置实际值回归】【工具默认】无参数，验证默认截图显示选区、文件编辑显示选择标注，并保持原存储值为空
#[test]
fn unset_tools_show_the_actual_entry_point_defaults() {
    let values = serde_json::to_value(crate::config::Config::default()).unwrap();
    for key in ["default_tool", "normal_tool", "fullscreen_default_tool"] {
        let field = entry(&format!("startup.{key}"));
        assert_eq!(field.display(&values), "selection");
        assert_eq!(field.display_label(&values, true), "选区");
        assert_eq!(field.configured_display(&values), "");
    }
    assert_eq!(entry("startup.file_default_tool").display(&values), "pick");
}

/// 【设置实际值回归】【工具优先级】无参数，验证共同、普通、全屏及文件工具实际覆盖顺序，返回值为空
#[test]
fn tools_resolve_common_fullscreen_and_file_overrides() {
    let mut values = json!({"startup":{"default_tool":"rect","normal_tool":"text"}});
    assert_eq!(entry("startup.default_tool").display(&values), "rectangle");
    assert_eq!(entry("startup.normal_tool").display(&values), "text");
    assert_eq!(
        entry("startup.fullscreen_default_tool").display(&values),
        "rectangle"
    );
    assert_eq!(
        entry("startup.file_default_tool").display(&values),
        "rectangle"
    );
    values["startup"]["fullscreen_default_tool"] = "pen".into();
    assert_eq!(entry("startup.file_default_tool").display(&values), "pen");
    values["startup"]["file_default_tool"] = "arrow".into();
    assert_eq!(entry("startup.file_default_tool").display(&values), "arrow");
    values["startup"]["default_tool"] = Value::Null;
    values["startup"]["fullscreen_default_tool"] = Value::Null;
    values["startup"]["file_default_tool"] = Value::Null;
    assert_eq!(
        entry("startup.fullscreen_default_tool").display(&values),
        "text"
    );
    assert_eq!(entry("startup.file_default_tool").display(&values), "text");
}

/// 【设置实际值回归】【目录与颜色】无参数，验证默认目录与生产目录规则相同，实际颜色可解析且显式颜色保持原值
#[test]
fn inherited_paths_and_colors_are_concrete_without_changing_storage() {
    let values = serde_json::to_value(crate::config::Config::default()).unwrap();
    let history = crate::config::history::History::default().path().unwrap();
    assert_eq!(
        entry("history.directory").display(&values),
        history.to_string_lossy()
    );
    let recording = dirs::video_dir()
        .or_else(dirs::home_dir)
        .unwrap()
        .join("Mark Shot");
    assert_eq!(
        entry("recording.directory").display(&values),
        recording.to_string_lossy()
    );
    let pictures = dirs::picture_dir()
        .or_else(|| dirs::home_dir().map(|home| home.join("Pictures")))
        .unwrap();
    assert_eq!(
        entry("save.directory").display(&values),
        pictures.join("screenshots").to_string_lossy()
    );
    assert_eq!(
        entry("log.path").display(&values),
        dirs::state_dir()
            .unwrap()
            .join("mark-shot/mark-shot.log")
            .to_string_lossy()
    );
    assert!(
        crate::theme::Rgba::parse_hex(&entry("startup.default_color").display(&values)).is_some()
    );
    assert_eq!(
        entry("startup.default_color").display(&json!({"startup":{"default_color":"#12345680"}})),
        "#12345680"
    );
    assert_eq!(entry("history.directory").configured_display(&values), "");
    assert_eq!(entry("recording.directory").configured_display(&values), "");
}

/// 【设置实际值回归】【全目录显示】无参数，验证中英文全部默认字段都有实际标签，字段说明不再出现继承占位
#[test]
fn every_default_field_has_a_concrete_label_in_both_languages() {
    let values = serde_json::to_value(crate::config::Config::default()).unwrap();
    for field in ENTRIES
        .iter()
        .filter(|entry| !matches!(entry.kind, Kind::Link(_)))
    {
        for chinese in [true, false] {
            let label = field.display_label(&values, chinese);
            assert!(!label.is_empty(), "{}", field.path());
            let note = field.note(&json!({}), &values, chinese);
            assert!(
                !format!("{label} {note}").contains("继承默认"),
                "{}",
                field.path()
            );
            assert!(!note.contains("Inherited default"), "{}", field.path());
        }
    }
    assert_eq!(
        entry("save.month_format").display_label(&json!({"save":{"month_format":""}}), true),
        "不使用子目录"
    );
}
