//! 【普通标注旋转验证】【原版几何】实际标注方法对照生产 Qt 返回的四角与旋转点

use super::*;
use mark_shot::features::annotation_rotation::geometry;

/// 【普通标注旋转验证】【坐标解码】按 value 返回原版记录的二维坐标，不用当前计算生成预期
fn point(value: &serde_json::Value) -> (f64, f64) {
    (value[0].as_f64().unwrap(), value[1].as_f64().unwrap())
}

/// 【普通标注旋转验证】【实际控制点】72个生产Qt参考逐项核对标注四角与旋转点，无参数、无返回值
#[test]
fn annotation_pose_matches_all_production_qt_rectangle_controls() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/qt-annotation-selection/reference.json"
    ))
    .unwrap();
    assert_eq!(reference["samples"].as_array().unwrap().len(), 72);
    for sample in reference["samples"].as_array().unwrap() {
        let values = &sample["bounds"];
        let bounds = Rect::from_xywh(
            values[0].as_f64().unwrap() as f32,
            values[1].as_f64().unwrap() as f32,
            values[2].as_f64().unwrap() as f32,
            values[3].as_f64().unwrap() as f32,
        )
        .unwrap();
        let mut annotation = rectangle(bounds);
        annotation.rotation_degrees = sample["angle"].as_f64().unwrap();
        for (actual, expected) in annotation
            .generic_frame_corners()
            .unwrap()
            .into_iter()
            .zip(sample["corners"].as_array().unwrap())
        {
            let expected = point(expected);
            assert!(
                (f64::from(actual.0) - expected.0).abs() < 0.0001,
                "{sample}"
            );
            assert!(
                (f64::from(actual.1) - expected.1).abs() < 0.0001,
                "{sample}"
            );
        }
        let actual = annotation.generic_rotation_handle().unwrap().1;
        let expected = point(&sample["rotation"]);
        assert!(
            (f64::from(actual.0) - expected.0).abs() < 0.0001,
            "{sample}"
        );
        assert!(
            (f64::from(actual.1) - expected.1).abs() < 0.0001,
            "{sample}"
        );
    }
}

/// 【普通标注旋转验证】【半圈语义】一般归一化使用fmod边界，Shift吸附使用remainder边界，无参数、无返回值
#[test]
fn normalization_and_snap_keep_the_distinct_qt_half_turn_rules() {
    for (input, expected) in [
        (540.0, 180.0),
        (-540.0, -180.0),
        (721.0, 1.0),
        (-541.0, 179.0),
    ] {
        assert_eq!(geometry::normalized(input), expected);
    }
    for (input, expected) in [
        (22.5, 45.0),
        (-22.5, -45.0),
        (180.0, 180.0),
        (540.0, -180.0),
        (-540.0, 180.0),
    ] {
        assert_eq!(geometry::snapped(input), expected);
    }
}

/// 【普通标注旋转验证】【实际命中】旋转后使用反向正文坐标命中，不能仍以原始轴对齐矩形选中，无参数、无返回值
#[test]
fn initial_hit_testing_follows_the_rotated_body() {
    let mut annotation = rectangle(Rect::from_xywh(100.0, 100.0, 160.0, 40.0).unwrap());
    annotation.rotation_degrees = 90.0;
    assert!(annotation.initial_hit_test((180.0, 60.0)));
    assert!(!annotation.initial_hit_test((110.0, 120.0)));
}

/// 【普通标注旋转验证】【导出映射】缩放和平移保留角度，坐标变换与原始点的同倍率映射一致，无参数、无返回值
#[test]
fn scaled_and_translated_annotations_keep_the_rotation_pose() {
    let mut annotation = rectangle(Rect::from_xywh(80.0, 60.0, 100.0, 80.0).unwrap());
    annotation.rotation_degrees = 45.0;
    let point = annotation.to_visual((80.0, 60.0));
    let mut scaled = annotation.scaled(2.0);
    assert_eq!(scaled.rotation_degrees, 45.0);
    assert_eq!(
        scaled.to_visual((160.0, 120.0)),
        (point.0 * 2.0, point.1 * 2.0)
    );
    scaled.translate_mut(12.0, -8.0);
    assert_eq!(scaled.rotation_degrees, 45.0);
    let moved = scaled.to_visual((172.0, 112.0));
    assert!((moved.0 - point.0 * 2.0 - 12.0).abs() < 0.0001);
    assert!((moved.1 - point.1 * 2.0 + 8.0).abs() < 0.0001);
}

/// 【普通标注旋转验证】【边缘轴心】画笔原始边界先裁到冻结画布，缩放同步画布，移动仍使用固定画布，无参数、无返回值
#[test]
fn clipped_pen_frame_defines_rotation_center_and_keeps_canvas_fixed() {
    let mut annotation = annotation(AnnotationShape::Pen {
        points: vec![(2.0, 20.0), (30.0, 40.0)],
    });
    let canvas = Rect::from_xywh(0.0, 0.0, 100.0, 80.0).unwrap();
    annotation.frozen_bounds = Some(canvas);
    annotation.rotation_degrees = 90.0;
    assert_eq!(annotation.rotation_center(), (19.0, 30.0));
    let scaled = annotation.scaled(1.5);
    assert_eq!(scaled.rotation_center(), (28.5, 45.0));
    assert_eq!(scaled.frozen_bounds.unwrap().width(), 150.0);
    annotation.translate_mut(10.0, 0.0);
    assert_eq!(annotation.frozen_bounds, Some(canvas));
    assert_eq!(annotation.rotation_center(), (26.0, 30.0));
}
