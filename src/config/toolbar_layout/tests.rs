//! 【工具栏回归】【全屏合并】验证顺序、重复按钮和旧配置兼容

use super::*;
use serde_json::json;

/// 【工具栏回归】【测试列表】按 names 解析稳定按钮标识，返回用于断言的列表
fn list(names: &[&str]) -> Vec<ToolbarItem> {
    names
        .iter()
        .map(|name| catalog::parse(name).unwrap())
        .collect()
}

/// 【工具栏回归】【合并顺序】无参数，主栏优先、侧栏追加，去除重复按钮与多余分隔线
#[test]
fn fullscreen_merges_bars_and_normalizes_separators() {
    let mut layout = ToolbarLayout {
        main: list(&[
            "separator",
            "copy",
            "separator",
            "separator",
            "undo",
            "separator",
        ]),
        actions: list(&[
            "separator",
            "copy",
            "separator",
            "save_as",
            "separator",
            "undo",
            "separator",
        ]),
        ..Default::default()
    };
    assert_eq!(
        layout.merged(),
        list(&["copy", "separator", "undo", "separator", "save_as"])
    );
    layout.actions = list(&["copy", "undo"]);
    assert_eq!(layout.merged(), list(&["copy", "separator", "undo"]));
    layout.main = list(&["redo"]);
    assert_eq!(
        Profile::Fullscreen.items(&layout).as_ref(),
        list(&["redo", "separator", "copy", "undo"])
    );
    assert!(!Profile::Fullscreen.vertical(&layout));
    layout.main_vertical = true;
    assert!(Profile::Fullscreen.vertical(&layout));
}

/// 【工具栏回归】【旧配置兼容】无参数，旧全屏字段不参与布局，主侧栏自定义保留且序列化只有两栏
#[test]
fn legacy_fullscreen_fields_do_not_override_the_two_bars() {
    let source = json!({"main":["copy"], "actions":["undo"], "fullscreen":["redo"], "fullscreen_vertical":true});
    let json_layout: ToolbarLayout = serde_json::from_value(source).unwrap();
    let toml_layout: ToolbarLayout = toml::from_str(
        "main = ['copy']\nactions = ['undo']\nfullscreen = ['redo']\nfullscreen_vertical = true",
    )
    .unwrap();
    assert_eq!(json_layout, toml_layout);
    assert_eq!(json_layout.merged(), list(&["copy", "separator", "undo"]));
    assert_eq!(Profile::ALL, [Profile::Main, Profile::Actions]);
    let serialized = serde_json::to_value(json_layout).unwrap();
    assert!(serialized.get("fullscreen").is_none());
    assert!(serialized.get("fullscreen_vertical").is_none());
}

/// 【工具栏回归】【录屏默认升级】旧完整默认主栏自动加入录屏，用户明确配置的精简主栏保持原样
#[test]
fn legacy_default_main_gains_recording_without_changing_custom_layouts() {
    let legacy = ITEMS
        .iter()
        .copied()
        .filter(|item| catalog::id(*item) != "recording")
        .collect::<Vec<_>>();
    let old = serde_json::to_value(ToolbarLayout {
        main: legacy,
        ..Default::default()
    })
    .unwrap();
    let upgraded: ToolbarLayout = serde_json::from_value(old).unwrap();
    assert_eq!(upgraded.main, ITEMS);
    let custom: ToolbarLayout =
        serde_json::from_value(json!({"main":["selection","rectangle"]})).unwrap();
    assert_eq!(custom.main, list(&["selection", "rectangle"]));
}
