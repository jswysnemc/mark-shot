//! 【niri 窗口识别回归】【自定义面板】验证旧版双屏偏移、有界推算与新版精确坐标

use super::{geometry, layout, model, panels};
use serde_json::{Value, json};

/// 【niri 窗口识别回归】【默认面板】无参数，返回不读取用户环境的空面板设置
fn settings() -> panels::Settings {
    panels::Settings {
        dms: Value::Null,
        waybar: Value::Null,
        edge: None,
        size: None,
    }
}

/// 【niri 窗口识别回归】【表面构造】按 namespace、output 与 layer 返回测试面板
fn surface(namespace: &str, output: &str, layer: &str) -> model::Layer {
    model::Layer {
        namespace: namespace.into(),
        output: Some(output.into()),
        layer: Some(layer.into()),
    }
}

/// 【niri 窗口识别回归】【真实偏移】无参数，验证报告中的双屏坐标与已有精确坐标保持一致，无返回值
#[test]
fn clavis_panel_restores_both_output_offsets_without_changing_exact_positions() {
    for (height, output_y, tile_height, expected_y) in [
        (1440.0, 720.0, 1368.0_f64, 782.0),
        (1067.33, 1093.0, 995.33, 1155.0),
    ] {
        for position in [Value::Null, json!([10.0, 62.0])] {
            let outputs = serde_json::from_value(json!({"DP-1":{"logical":{
                "x":0.0,"y":output_y,"width":1707.0,"height":height
            }}}))
            .unwrap();
            let workspaces = serde_json::from_value::<Vec<model::Workspace>>(json!([{
                "id":1,"output":"DP-1","is_active":true,"active_window_id":1
            }]))
            .unwrap();
            let windows = serde_json::from_value::<Vec<model::Window>>(json!([{
                "id":1,"workspace_id":1,"layout":{
                    "pos_in_scrolling_layout":[0,0],"tile_pos_in_workspace_view":position,
                    "tile_size":[800.0,tile_height],"window_size":[800.0,tile_height]
                }
            }]))
            .unwrap();
            let actual = geometry::build(
                &outputs,
                &workspaces,
                &windows,
                &[surface("clavis-shell-bar-horizontal", "DP-1", "Top")],
                &layout::Layout {
                    gaps: 10.0,
                    struts: [0.0; 4],
                },
                &settings(),
            );
            assert_eq!(actual.len(), 1);
            assert!((f64::from(actual[0].rect.top()) - expected_y).abs() < 0.01);
            assert_eq!(
                f64::from(actual[0].rect.height()),
                tile_height.round_ties_even()
            );
        }
    }
}

/// 【niri 窗口识别回归】【保守推算】无参数，验证方向、层级、输出、占用上限和显式覆盖，无返回值
#[test]
fn custom_panel_inference_requires_matching_horizontal_surface_and_bounded_height() {
    let layout = layout::Layout {
        gaps: 10.0,
        struts: [4.0, 6.0, 0.0, 0.0],
    };
    for (name, output, layer, tallest, expected) in [
        ("ags-panel-top", "DP-1", "Top", 750.0, [40.0, 0.0, 0.0, 0.0]),
        ("eww-dock", "DP-1", "Overlay", 750.0, [0.0, 40.0, 0.0, 0.0]),
        (
            "quickshell-bar-bottom",
            "DP-1",
            "Top",
            750.0,
            [0.0, 40.0, 0.0, 0.0],
        ),
        ("panel-left", "DP-1", "Top", 750.0, [0.0; 4]),
        ("panel-top", "DP-2", "Top", 750.0, [0.0; 4]),
        ("panel-top", "DP-1", "Bottom", 750.0, [0.0; 4]),
        ("notification", "DP-1", "Overlay", 750.0, [0.0; 4]),
        ("panel-top", "DP-1", "Top", 500.0, [0.0; 4]),
        ("panel-top", "DP-1", "Top", f64::NAN, [0.0; 4]),
    ] {
        assert_eq!(
            panels::insets(
                &settings(),
                &[surface(name, output, layer)],
                "DP-1",
                &layout,
                800.0,
                Some(tallest)
            ),
            expected
        );
    }
    let layers = [
        surface("panel-top", "DP-1", "Top"),
        surface("dock-bottom", "DP-1", "Top"),
    ];
    assert_eq!(
        panels::insets(&settings(), &layers, "DP-1", &layout, 800.0, Some(750.0)),
        [0.0; 4]
    );
    let mut manual = settings();
    manual.edge = Some("top".into());
    manual.size = Some(52.0);
    assert_eq!(
        panels::insets(&manual, &layers, "DP-1", &layout, 800.0, Some(750.0)),
        [52.0, 0.0, 0.0, 0.0]
    );
    manual.edge = Some("none".into());
    assert_eq!(
        panels::insets(&manual, &layers, "DP-1", &layout, 800.0, Some(750.0)),
        [0.0; 4]
    );
}
