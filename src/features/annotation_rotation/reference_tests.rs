//! 【普通标注旋转对照】【实际样本】核对完整生产Qt函数的角度和缩放结果，包含原始像素比例映射

use super::{geometry, resize};
use crate::types::SelectionHandle;
use serde_json::Value;
use tiny_skia::Rect;

/// 【普通标注旋转对照】【来源读取】返回生产探针原始结果，不使用当前实现生成预期，无参数
fn reference() -> Value {
    serde_json::from_str(include_str!(
        "../../../tests/fixtures/qt-annotation-rotation/reference.json"
    ))
    .unwrap()
}

/// 【普通标注旋转对照】【二维读取】按 value 返回原始双精度点
fn point(value: &Value) -> (f64, f64) {
    (value[0].as_f64().unwrap(), value[1].as_f64().unwrap())
}

/// 【普通标注旋转对照】【阈值与角度】一般归一化、Shift吸附和0.1像素方向阈值逐项等于Qt生产计算，无参数、无返回值
#[test]
fn production_qt_angles_and_subpixel_rotation_radius_match() {
    let reference = reference();
    assert_eq!(reference["angles"].as_array().unwrap().len(), 18);
    assert_eq!(reference["drags"].as_array().unwrap().len(), 72);
    for sample in reference["angles"].as_array().unwrap() {
        let input = sample["input"].as_f64().unwrap();
        assert_eq!(
            geometry::normalized(input),
            sample["normalized"].as_f64().unwrap()
        );
        assert_eq!(
            geometry::snapped(input),
            sample["snapped"].as_f64().unwrap()
        );
    }
    for sample in reference["drags"].as_array().unwrap() {
        let actual = geometry::drag_degrees(
            point(&sample["center"]),
            point(&sample["start"]),
            point(&sample["current"]),
            0.1,
        );
        if let Some(expected) = sample["degrees"].as_f64() {
            assert!((actual.unwrap() - expected).abs() < 1e-10, "{sample}");
        } else {
            assert!(actual.is_none(), "{sample}");
        }
    }
}

/// 【普通标注旋转对照】【实际缩放】384项八方向结果在四种原图比例及偏移画布逐项对照，无参数、无返回值
#[test]
fn production_qt_resize_matches_at_fractional_scales_and_negative_origins() {
    let reference = reference();
    let samples = reference["resize"].as_array().unwrap();
    assert_eq!(samples.len(), 384);
    let handles = [
        SelectionHandle::TopLeft,
        SelectionHandle::Top,
        SelectionHandle::TopRight,
        SelectionHandle::Right,
        SelectionHandle::BottomRight,
        SelectionHandle::Bottom,
        SelectionHandle::BottomLeft,
        SelectionHandle::Left,
    ];
    for sample in samples {
        for scale in [1.0_f32, 1.25, 1.5, 2.0] {
            for origin in [(0.0_f32, 0.0_f32), (-80.0, 20.0)] {
                let values = &sample["before"];
                let before = Rect::from_xywh(
                    values[0].as_f64().unwrap() as f32 / scale + origin.0,
                    values[1].as_f64().unwrap() as f32 / scale + origin.1,
                    values[2].as_f64().unwrap() as f32 / scale,
                    values[3].as_f64().unwrap() as f32 / scale,
                )
                .unwrap();
                let pointer = point(&sample["pointer"]);
                let actual = resize::resized(
                    before,
                    handles[sample["handle"].as_u64().unwrap() as usize],
                    (
                        pointer.0 as f32 / scale + origin.0,
                        pointer.1 as f32 / scale + origin.1,
                    ),
                    Rect::from_xywh(origin.0, origin.1, 640.0 / scale, 480.0 / scale).unwrap(),
                    scale,
                    sample["aspect"].as_bool().unwrap(),
                )
                .unwrap();
                for (index, actual) in [
                    (actual.x() - origin.0) * scale,
                    (actual.y() - origin.1) * scale,
                    actual.width() * scale,
                    actual.height() * scale,
                ]
                .into_iter()
                .enumerate()
                {
                    let expected = sample["result"][index].as_f64().unwrap();
                    assert!(
                        (f64::from(actual) - expected).abs()
                            < f64::from(f32::EPSILON) * 640.0 * 8.0,
                        "scale={scale} origin={origin:?} actual={actual} expected={expected} sample={sample}"
                    );
                }
            }
        }
    }
}
