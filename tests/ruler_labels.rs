//! 【尺子测试】【说明布局】验证测量完整性、边界约束、避让与比例一致

use mark_shot::features::ruler::labels::{clamped, prepare_all};
use mark_shot::features::ruler::{Measurement, RulerGeometry, RulerMode};
use tiny_skia::Rect;

/// 【尺子测试】【长度构造】使用 start、end 构造源图像素比例为 2 的测量
fn ruler(start: (f32, f32), end: (f32, f32)) -> Measurement {
    let mut measurement = Measurement::new(RulerMode::Length, start, 2.0).unwrap();
    measurement.geometry = RulerGeometry::Length {
        start,
        end,
        rotation: 0.0,
    };
    measurement
}

/// 【尺子测试】【完整说明】原版rulerLabelSize与clampRulerFloatingRect在窄范围内保留完整单行，只平移不缩小
#[test]
fn narrow_labels_keep_all_measurement_fields() {
    let measurement = ruler((30.0, 30.0), (200.0, 150.0));
    let bounds = Rect::from_xywh(0.0, 0.0, 130.0, 120.0).unwrap();
    let mut fonts = mark_shot::fonts::bundled();
    let labels = prepare_all(std::iter::once((measurement, 3.0)), bounds, &mut fonts);
    assert_eq!(labels.len(), 1);
    assert_eq!(
        labels[0].lines,
        ["340 x 240 px   diag 416 px   area 81600 px"]
    );
    assert!(labels[0].rect.width() > bounds.width());
    assert_eq!(labels[0].rect.left(), 8.0 * labels[0].unit);
    assert_eq!(labels[0].font_size, 17.0);
}

/// 【尺子测试】【说明避让】有足够可用空间时，各长度说明避开其他尺子刻度与已放置说明
#[test]
fn length_labels_avoid_each_other_and_other_rulers_when_space_allows() {
    let rulers = [
        ruler((100.0, 100.0), (280.0, 240.0)),
        ruler((100.0, 335.0), (260.0, 475.0)),
    ];
    let bounds = Rect::from_xywh(0.0, 0.0, 1100.0, 800.0).unwrap();
    let mut fonts = mark_shot::fonts::bundled();
    let labels = prepare_all(
        rulers.into_iter().map(|ruler| (ruler, 3.0)),
        bounds,
        &mut fonts,
    );
    assert_eq!(labels.len(), 2);
    assert!(labels[0].rect.intersect(&labels[1].rect).is_none());
    for label in labels {
        for ruler in rulers {
            let body = mark_shot::features::ruler::tick_layout::Layout::new(
                ruler,
                3.0,
                Some(bounds),
                Default::default(),
            )
            .unwrap()
            .scale_bounds()
            .unwrap();
            assert!(
                body.intersect(&label.rect).is_none(),
                "body={body:?}, label={label:?}"
            );
        }
    }
}

/// 【尺子测试】【原图说明比例】原版rulerFont按绘制目标取整，导出保留测量文字与样式单位但不连续缩放字体度量
#[test]
fn exported_labels_preserve_measurement_text_and_scale_layout() {
    let measurement = ruler((100.0, 100.0), (250.0, 200.0));
    let mut fonts = mark_shot::fonts::bundled();
    let bounds = Rect::from_xywh(0.0, 0.0, 800.0, 600.0).unwrap();
    let original = prepare_all(std::iter::once((measurement, 12.0)), bounds, &mut fonts);
    let bounds = Rect::from_xywh(0.0, 0.0, 1600.0, 1200.0).unwrap();
    let exported = prepare_all(
        std::iter::once((measurement.scaled(2.0), 24.0)),
        bounds,
        &mut fonts,
    );
    assert_eq!(exported[0].lines, original[0].lines);
    assert_eq!(original[0].font_size, 43.0);
    assert_eq!(exported[0].font_size, 87.0);
    assert_eq!(exported[0].unit, original[0].unit * 2.0);
    assert!((exported[0].rect.width() - original[0].rect.width() * 2.0).abs() > 0.1);
}

/// 【尺子测试】【角度说明优先】角度说明先登记到布局，不完整角度不生成误导说明
#[test]
fn angle_labels_are_placed_before_length_labels_and_pending_angles_are_skipped() {
    let mut angle = Measurement::new(RulerMode::Angle, (500.0, 260.0), 2.0).unwrap();
    angle.geometry = RulerGeometry::Angle {
        points: [(500.0, 260.0), (650.0, 260.0), (500.0, 410.0)],
        count: 3,
    };
    let pending = Measurement::new(RulerMode::Angle, (100.0, 100.0), 2.0).unwrap();
    let mut fonts = mark_shot::fonts::bundled();
    let bounds = Rect::from_xywh(0.0, 0.0, 1000.0, 700.0).unwrap();
    let labels = prepare_all(
        [
            (ruler((100.0, 100.0), (260.0, 260.0)), 3.0),
            (pending, 3.0),
            (angle, 3.0),
        ]
        .into_iter(),
        bounds,
        &mut fonts,
    );
    assert_eq!(labels.len(), 2);
    assert_eq!(labels[0].lines, ["90.0\u{b0}"]);
    assert!(labels[1].lines[0].contains("diag"));
}

/// 【尺子测试】【越界说明】原版clampRulerFloatingRect平移超大说明到负坐标边界，不裁减尺寸
#[test]
fn floating_labels_clamp_to_negative_origin_bounds() {
    let bounds = Rect::from_xywh(-300.0, -150.0, 400.0, 200.0).unwrap();
    let rect = Rect::from_xywh(-800.0, 700.0, 900.0, 450.0).unwrap();
    let adjusted = clamped(rect, bounds, 8.0);
    assert_eq!(adjusted.left(), -292.0);
    assert_eq!(adjusted.top(), -142.0);
    assert_eq!(adjusted.width(), 900.0);
    assert_eq!(adjusted.height(), 450.0);
}
