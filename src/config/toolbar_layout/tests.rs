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
