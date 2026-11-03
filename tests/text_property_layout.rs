//! 【文字属性测试】【实际几何】使用原版 ShotWindow 控件记录验证属性栏和字体弹层尺寸，不以手写参考替代实际控件

use mark_shot::ui::{
    font_panel::layout::FontLayout,
    text_properties::layout::{Control, TextLayout},
};
use tiny_skia::Rect;

/// 【文字属性测试】【矩形记录】将 rect 转换为便于和 Qt 记录比较的整数数组
fn geometry(rect: Rect) -> Vec<i32> {
    vec![
        rect.left() as i32,
        rect.top() as i32,
        rect.width() as i32,
        rect.height() as i32,
    ]
}

#[test]
fn text_properties_match_actual_qt_controls() {
    let reference: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/qt-property-reference/geometry.json")).unwrap();
    for (name, editing) in [("properties", false), ("selected-properties", true)] {
        let layout = TextLayout::new(30.0, 11.0, 22.0, editing);
        assert_eq!(
            serde_json::json!([layout.size.0 as i32, layout.size.1 as i32]),
            reference[name]["size"]
        );
        let controls = reference[name]["controls"].as_array().unwrap();
        let indices = [2, 5, 6, 7, 8, 9];
        for (index, rect) in layout.controls.into_iter().enumerate() {
            if let Some(rect) = rect {
                assert_eq!(
                    serde_json::json!(geometry(rect)),
                    controls[indices[index]]["geometry"]
                );
            }
        }
        for (rect, index) in [
            (layout.glyphs[0], 0),
            (layout.values[0], 1),
            (layout.glyphs[1], 3),
            (layout.values[1], 4),
        ] {
            assert_eq!(
                serde_json::json!(geometry(rect)),
                controls[index]["geometry"]
            );
        }
    }
}

#[test]
fn labels_do_not_act_like_input_fields() {
    let layout = TextLayout::new(30.0, 11.0, 22.0, false);
    assert_eq!(layout.hit((140.0, 22.0)), None);
    assert_eq!(layout.hit((10.0, 22.0)), None);
    assert_eq!(layout.hit((36.0, 20.0)), Some(Control::Size));
    assert_eq!(layout.hit((400.0, 20.0)), Some(Control::Font));
    assert_eq!(layout.hit((436.0, 20.0)), None);
}

#[test]
fn slider_metrics_and_endpoints_match_qt_style() {
    let cell = Rect::from_xywh(0.0, 0.0, 88.0, 15.0).unwrap();
    assert_eq!(
        geometry(TextLayout::slider_handle(cell, 1, (-11, 281))),
        vec![3, 1, 14, 12]
    );
    assert_eq!(
        geometry(TextLayout::slider_handle(cell, 281, (-11, 281))),
        vec![74, 1, 14, 12]
    );
    assert_eq!(TextLayout::slider_value(cell, 74.0, (-11, 281)), 281);
    assert_eq!(TextLayout::slider_value(cell, -100.0, (-11, 281)), -11);
    assert_eq!(TextLayout::slider_value(cell, 100.0, (-11, 281)), 281);
}

#[test]
fn font_popup_retains_qt_partitions_when_search_filters_rows() {
    let layout = FontLayout::new(2366, (1067.0, 667.0));
    assert_eq!(layout.size, (360.0, 360.0));
    assert_eq!(geometry(layout.search), vec![6, 7, 348, 28]);
    assert_eq!(geometry(layout.list), vec![6, 40, 348, 280]);
    assert_eq!(geometry(layout.size_field), vec![6, 326, 288, 26]);
    assert_eq!(geometry(layout.bold), vec![298, 326, 26, 26]);
    assert_eq!(geometry(layout.italic), vec![328, 326, 26, 26]);
    let bounds = (400.0, 400.0);
    let properties = Rect::from_xywh(8.0, 350.0, 380.0, 44.0).unwrap();
    let button = Rect::from_xywh(350.0, 356.0, 32.0, 32.0).unwrap();
    let pos = layout.position(properties, button, bounds);
    assert!(pos.0 >= 8.0 && pos.0 + layout.size.0 <= 392.0);
    assert!(pos.1 >= 8.0 && pos.1 + layout.size.1 <= 392.0);
}
