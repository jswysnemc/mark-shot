//! 【画布导航测试】【完整生产对照】验证视图、原图坐标、滚动条和真实Qt滚轮分派，不生成替代预期

use mark_shot::{
    features::canvas_navigation::{
        coordinates::{Point, Rect, Size},
        model::Navigation,
        policy::{Policy, WheelTarget, wheel_factor},
        scrollbars::{Axis, Scrollbars},
    },
    tools::Tool,
    types::ScrollEvent,
};
use serde_json::Value;

/// 【画布导航测试】【原生参考】返回采集的完整Qt生产结果，无参数
fn reference() -> Value {
    serde_json::from_str(include_str!("fixtures/qt-canvas-navigation/reference.json")).unwrap()
}

/// 【画布导航测试】【双精度读取】按value返回原始坐标，不预先转换为绘制浮点类型
fn point(value: &Value) -> Point {
    (value[0].as_f64().unwrap(), value[1].as_f64().unwrap())
}

/// 【画布导航测试】【尺寸读取】按value返回原图或窗口的整数尺寸
fn size(value: &Value) -> Size {
    (
        value[0].as_u64().unwrap() as u32,
        value[1].as_u64().unwrap() as u32,
    )
}

/// 【画布导航测试】【浮点比较】按actual、expected与context检查十万分之一像素以内的差异，无返回值
fn close(actual: &[f64], expected: &Value, context: &str) {
    let expected = expected.as_array().unwrap();
    assert_eq!(actual.len(), expected.len(), "{context}");
    for (actual, expected) in actual.iter().zip(expected) {
        let expected = expected.as_f64().unwrap();
        assert!(
            (actual - expected).abs() <= 0.00001,
            "{context}: {actual} != {expected}"
        );
    }
}

/// 【画布导航测试】【矩形比较】按actual、expected与context比较原始左、上、宽、高，无返回值
fn rectangle(actual: Rect, expected: &Value, context: &str) {
    close(
        &[actual.x, actual.y, actual.width, actual.height],
        expected,
        context,
    );
}

/// 【画布导航测试】【原生轴比较】按actual、expected与context比较Qt滚动范围、页长、整点位置和显隐，无返回值
fn axis(actual: Axis, expected: &Value, context: &str) {
    rectangle(actual.rect, &expected["rect"], context);
    for (name, value) in [
        ("maximum", actual.maximum),
        ("page_step", actual.page_step),
        ("single_step", actual.single_step),
        ("value", actual.value),
    ] {
        assert_eq!(
            i64::from(value),
            expected[name].as_i64().unwrap(),
            "{context} {name}"
        );
    }
    assert_eq!(
        actual.visible,
        expected["visible"].as_bool().unwrap(),
        "{context} visible"
    );
}

/// 【画布导航测试】【完整视图比较】按view、bars、expected与context检查轴心、缩放、裁切坐标和两轴原生状态，无返回值
fn compare(view: Navigation, bars: Scrollbars, expected: &Value, context: &str) {
    close(
        &[view.zoom()],
        &serde_json::json!([expected["zoom"]]),
        context,
    );
    assert_eq!(
        view.center().is_some(),
        expected["initialized"].as_bool().unwrap(),
        "{context}"
    );
    let center = view.center().unwrap_or_default();
    close(&[center.0, center.1], &expected["center"], context);
    assert_eq!(view.window_size(), size(&expected["size"]), "{context}");
    rectangle(view.projection().widget_rect, &expected["rect"], context);
    let image = view
        .projection()
        .widget_to_image(point(&expected["probe"]))
        .unwrap();
    close(&[image.0, image.1], &expected["image_probe"], context);
    let widget = view.projection().image_to_widget(image).unwrap();
    close(&[widget.0, widget.1], &expected["widget_probe"], context);
    axis(bars.horizontal, &expected["horizontal"], context);
    axis(bars.vertical, &expected["vertical"], context);
}

/// 【画布导航测试】【连续原生几何】多种长图、单像素图及分数窗口连续缩放、平移、滚动和复位均匹配完整Qt函数
#[test]
fn every_qt_navigation_sequence_matches_geometry_coordinates_and_scrollbars() {
    for (scene, sample) in reference()["geometry"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        let mut available = true;
        let mut view =
            Navigation::new(size(&sample["image"]), size(&sample["size"]), available).unwrap();
        let mut bars = Scrollbars::default();
        bars.update(view, available, (100.0, 100.0), (false, false));
        compare(
            view,
            bars,
            &sample["initial"],
            &format!("scene={scene} initial"),
        );
        for (step, value) in sample["steps"].as_array().unwrap().iter().enumerate() {
            match value["action"].as_str().unwrap() {
                "zoom" => {
                    view.zoom_at(
                        value["factor"].as_f64().unwrap(),
                        point(&value["anchor"]),
                        available,
                    );
                }
                "pan" => {
                    view.pan_from(
                        point(&value["center"]),
                        point(&value["start"]),
                        point(&value["current"]),
                        available,
                    );
                }
                "scroll" => {
                    bars.apply_values(
                        &mut view,
                        (
                            value["values"][0].as_i64().unwrap() as i32,
                            value["values"][1].as_i64().unwrap() as i32,
                        ),
                        available,
                    );
                }
                "resize" => {
                    view.resize(size(&value["size"]), available);
                }
                "reset" => {
                    view.reset(available);
                }
                "availability" => {
                    available = value["available"].as_bool().unwrap();
                    view.refresh(available);
                }
                other => panic!("unknown production action {other}"),
            }
            let dragging = value["dragging"].as_array().map_or((false, false), |v| {
                (v[0].as_bool().unwrap(), v[1].as_bool().unwrap())
            });
            bars.update(view, available, point(&value["pointer"]), dragging);
            compare(
                view,
                bars,
                &value["after"],
                &format!("scene={scene} step={step} {}", value["action"]),
            );
        }
    }
}

/// 【画布导航测试】【图片入口】实际FileCanvas初始视口匹配Qt整体适配，小图和长图保留全部原始像素及透明度
#[test]
fn file_canvas_initial_layout_matches_qt_and_keeps_the_original_pixels() {
    for sample in reference()["geometry"].as_array().unwrap() {
        let image_size = size(&sample["image"]);
        let window_size = size(&sample["size"]);
        let mut image = tiny_skia::Pixmap::new(image_size.0, image_size.1).unwrap();
        image.fill(tiny_skia::Color::from_rgba8(80, 120, 160, 180));
        let original = image.data().to_vec();
        let bounds =
            tiny_skia::Rect::from_xywh(-173.0, 49.0, window_size.0 as f32, window_size.1 as f32)
                .unwrap();
        let file = mark_shot::editor::file_canvas::FileCanvas::new(image, bounds).unwrap();
        let expected = &sample["initial"]["rect"];
        for (actual, expected) in [
            f64::from(file.viewport.x() - bounds.x()),
            f64::from(file.viewport.y() - bounds.y()),
            f64::from(file.viewport.width()),
            f64::from(file.viewport.height()),
        ]
        .iter()
        .zip(expected.as_array().unwrap())
        {
            assert!(
                (actual - expected.as_f64().unwrap()).abs() < 0.0001,
                "image={image_size:?} window={window_size:?}"
            );
        }
        let crop = file.crop(file.viewport).unwrap();
        assert_eq!(crop.pixels.data(), original);
        assert_eq!(file.original.data(), original);
    }
}

/// 【画布导航测试】【工具输入适配】按name返回采集样本对应的实际工具，不运行分派逻辑生成预期
fn tool(name: &str) -> Tool {
    match name {
        "selection" => Tool::Selection,
        "pick" => Tool::Pick,
        "pen" => Tool::Pen,
        "line" => Tool::Line,
        "highlighter" => Tool::Highlighter,
        "rectangle" => Tool::Rectangle,
        "circle" => Tool::Circle,
        "marker" => Tool::Marker,
        "arrow" => Tool::Arrow,
        "magnifier" => Tool::Magnifier,
        "laser" => Tool::Laser,
        "ruler" => Tool::Ruler,
        "text" => Tool::Text,
        "redact" => Tool::Redact,
        "number" => Tool::Number,
        other => panic!("unknown production tool {other}"),
    }
}

/// 【画布导航测试】【真实滚轮职责】完整Qt wheelEvent的真实缩放或宽度调用，与工具、成员、阶段和拖动归属逐项一致
#[test]
fn every_qt_wheel_route_preserves_selected_annotation_priority() {
    for sample in reference()["wheel"].as_array().unwrap() {
        let policy = Policy {
            image_enabled: sample["enabled"].as_bool().unwrap(),
            editing: sample["editing"].as_bool().unwrap(),
        };
        let tool = tool(sample["tool"].as_str().unwrap());
        assert_eq!(policy.available(), sample["available"].as_bool().unwrap());
        assert_eq!(
            policy.wheel_zooms(tool),
            sample["wheel_zooms"].as_bool().unwrap()
        );
        let target = policy.wheel_target(
            tool,
            sample["selected"].as_u64().unwrap() as usize,
            sample["editing_existing"].as_bool().unwrap_or(false),
            sample["blocked"].as_bool().unwrap(),
            ScrollEvent::wheel(120),
        );
        let expected = match sample["route"].as_str().unwrap() {
            "zoom" => WheelTarget::ImageZoom,
            "width" => WheelTarget::AnnotationWidth,
            "none" => WheelTarget::None,
            other => panic!("unknown production route {other}"),
        };
        assert_eq!(target, expected, "{sample}");
        assert_eq!(
            sample["history"], 0,
            "navigation captured history: {sample}"
        );
    }
}

/// 【画布导航测试】【角度像素优先】高分辨率角度与纯像素连续倍率逐项匹配生产Qt计算，水平轴不改变纵向缩放
#[test]
fn every_qt_wheel_factor_matches_angle_and_pixel_priority() {
    for sample in reference()["factor"].as_array().unwrap() {
        let mut event = ScrollEvent::pixels(13.0, sample["pixel"].as_f64().unwrap());
        event.vertical.angle_delta = Some(sample["angle"].as_i64().unwrap() as i32);
        close(
            &[wheel_factor(event)],
            &serde_json::json!([sample["factor"]]),
            "wheel factor",
        );
    }
}
