//! 【niri 窗口识别回归】【原版几何】使用实际 Python 原版输出验证窗口、列布局和独占区

use super::*;
use serde_json::{Value, json};

/// 【niri 窗口识别回归】【设置构造】按 dms 返回无用户环境依赖的面板设置
fn settings(dms: Value) -> panels::Settings {
    panels::Settings {
        dms,
        waybar: Value::Null,
        edge: None,
        size: None,
    }
}

/// 【niri 窗口识别回归】【矩形编码】按 windows 返回可排序的完整整数矩形列表
fn encoded(windows: &[Window]) -> Vec<[i32; 4]> {
    let mut rects: Vec<_> = windows
        .iter()
        .map(|window| {
            assert!(window.z_order.is_none());
            [
                window.rect.left() as i32,
                window.rect.top() as i32,
                window.rect.width() as i32,
                window.rect.height() as i32,
            ]
        })
        .collect();
    rects.sort_unstable();
    rects
}

/// 【niri 窗口识别回归】【实际数据】验证九十六个原版精确、旧版、全屏、活动列和 DMS 场景，无参数、无返回值
#[test]
fn geometry_matches_original_niri_script() {
    let data: Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/qt-niri-window-detection/geometry.json"
    ))
    .unwrap();
    for case in data["cases"].as_array().unwrap() {
        let outputs = serde_json::from_value(case["outputs"].clone()).unwrap();
        let workspaces =
            serde_json::from_value::<Vec<model::Workspace>>(case["workspaces"].clone()).unwrap();
        let windows =
            serde_json::from_value::<Vec<model::Window>>(case["windows"].clone()).unwrap();
        let layers = serde_json::from_value::<Vec<model::Layer>>(case["layers"].clone()).unwrap();
        let layout = layout::Layout {
            gaps: case["layout"]["gaps"].as_f64().unwrap(),
            struts: ["strut_top", "strut_bottom", "strut_left", "strut_right"]
                .map(|name| case["layout"][name].as_f64().unwrap()),
        };
        let actual = geometry::build(
            &outputs,
            &workspaces,
            &windows,
            &layers,
            &layout,
            &settings(case["dms"].clone()),
        );
        let mut expected =
            serde_json::from_value::<Vec<[i32; 4]>>(case["expected"].clone()).unwrap();
        expected.sort_unstable();
        assert_eq!(encoded(&actual), expected, "{}", case["name"]);
    }
}

/// 【niri 窗口识别回归】【面板反推】验证未知水平面板尺寸、显式覆盖与独占关闭，无参数、无返回值
#[test]
fn unknown_waybar_height_uses_tallest_column_and_overrides_are_explicit() {
    let layers = vec![model::Layer {
        namespace: "waybar".into(),
        output: Some("DP-1".into()),
    }];
    let layout = layout::Layout {
        gaps: 8.0,
        struts: [4.0, 6.0, 0.0, 0.0],
    };
    let mut settings = settings(Value::Null);
    assert_eq!(
        panels::insets(&settings, &layers, "DP-1", &layout, 800.0, Some(750.0)),
        [40.0, 0.0, 0.0, 0.0]
    );
    settings.waybar = json!({"exclusive":false});
    assert_eq!(
        panels::insets(&settings, &layers, "DP-1", &layout, 800.0, Some(750.0)),
        [0.0; 4]
    );
    settings.edge = Some("bottom".into());
    settings.size = Some(24.0);
    assert_eq!(
        panels::insets(&settings, &[], "DP-1", &layout, 800.0, None),
        [0.0, 24.0, 0.0, 0.0]
    );
    settings.edge = Some("none".into());
    assert_eq!(
        panels::insets(&settings, &layers, "DP-1", &layout, 800.0, Some(750.0)),
        [0.0; 4]
    );
}

/// 【niri 窗口识别回归】【配置展开】验证循环 include、相对路径、注释与无关嵌套节点，无参数、无返回值
#[test]
fn layout_config_preserves_nested_scope_and_comment_strings() {
    let text = layout::uncomment(
        "// header\nlayout {\n gaps 8\n struts {\n top 4\n left -8\n }\n}\nwindow-rule {\n gaps 99\n}\nname \"https://example/path\"\n",
    );
    let parsed = layout::parse(&text);
    assert_eq!(parsed.gaps, 8.0);
    assert_eq!(parsed.struts, [4.0, 0.0, -8.0, 0.0]);
    assert!(text.contains("https://example/path"));
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("large.kdl");
    std::fs::write(&path, "0123456789").unwrap();
    assert!(layout::bounded_text(&path, &mut 4).is_none());
    let first = directory.path().join("first.kdl");
    let second = directory.path().join("second.kdl");
    std::fs::write(&first, "layout {\n include \"second.kdl\"\n}\n").unwrap();
    std::fs::write(&second, "include \"first.kdl\"\ngaps 12\n").unwrap();
    let expanded = layout::expand(&first, &mut std::collections::HashSet::new(), &mut 2048, 0);
    assert_eq!(layout::parse(&expanded).gaps, 12.0);
}
