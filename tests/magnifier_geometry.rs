//! 【放大镜测试】【几何契约】验证原图单位、双框比例与边界，不启动图形环境

use mark_shot::features::magnifier::{Magnifier, MagnifierShape};
use tiny_skia::Rect;

/// 【放大镜测试】【测试构造】用 shape、scale 创建带负坐标的空放大镜；返回有效模型
fn draft(shape: MagnifierShape, scale: f32) -> Magnifier {
    Magnifier::new(
        shape,
        (100.0, 180.0),
        2.75,
        Rect::from_xywh(-80.0, 20.0, 1000.0, 600.0).unwrap(),
        scale,
    )
    .unwrap()
}

/// 【放大镜测试】【近似比较】验证 actual 与 expected 的误差小于千分之一；失败时显示两值
fn near(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.001, "{actual} != {expected}");
}

/// 【放大镜测试】【完整框约束】验证 frame 的四边均位于 bounds 内；允许千分之一的浮点误差
fn inside(bounds: Rect, frame: Rect) {
    assert!(frame.left() >= bounds.left() - 0.001);
    assert!(frame.top() >= bounds.top() - 0.001);
    assert!(frame.right() <= bounds.right() + 0.001);
    assert!(frame.bottom() <= bounds.bottom() + 0.001);
}

/// 【放大镜测试】【原图门槛】最短拖动、最小直径与最大直径均使用原图像素，不使用屏幕逻辑尺寸
#[test]
fn creation_threshold_and_diameter_use_original_pixels() {
    let empty = draft(MagnifierShape::Circle, 2.0);
    assert!(!empty.draft_to((131.9, 180.0)).measures());
    let minimum = empty.draft_to((132.0, 180.0));
    near(minimum.size.0, 36.0);
    near(minimum.size.1, 36.0);
    let maximum = empty.draft_to((500.0, 180.0));
    near(maximum.size.0, 160.0);
    near(maximum.size.1, 160.0);
}

/// 【放大镜测试】【小图约束】小于最小直径的原图仍可创建，两个完整框均保持在原图内
#[test]
fn small_images_limit_both_frames_without_changing_zoom() {
    let bounds = Rect::from_xywh(-10.0, 20.0, 50.0, 40.0).unwrap();
    let magnifier = Magnifier::new(MagnifierShape::Circle, (-10.0, 20.0), 6.0, bounds, 2.0)
        .unwrap()
        .draft_to((40.0, 60.0));
    assert!(magnifier.measures());
    near(magnifier.size.0, 40.0);
    for frame in [
        magnifier.lens_rect().unwrap(),
        magnifier.source_rect().unwrap(),
    ] {
        inside(bounds, frame);
    }
    near(
        magnifier.lens_rect().unwrap().width() / magnifier.source_rect().unwrap().width(),
        6.0,
    );
}

/// 【放大镜测试】【边缘双框】创建与倍率修改约束两个框的完整边界，不把源框裁切成错误比例
#[test]
fn edge_creation_and_zoom_preserve_the_full_source_rectangle() {
    for shape in MagnifierShape::ALL {
        for zoom in [1.25, 2.75, 6.0] {
            let bounds = Rect::from_xywh(-80.0, 20.0, 1000.0, 600.0).unwrap();
            let before = Magnifier::new(shape, (-80.0, 20.0), zoom, bounds, 1.6)
                .unwrap()
                .draft_to((920.0, 620.0));
            for value in [1.25, 2.75, 6.0] {
                let after = before.with_zoom(value);
                let lens = after.lens_rect().unwrap();
                let source = after.source_rect().unwrap();
                inside(bounds, lens);
                inside(bounds, source);
                near(lens.width() / source.width(), value);
                near(lens.height() / source.height(), value);
                assert_eq!(before.center, after.center);
                assert_eq!(before.size, after.size);
            }
        }
    }
}

/// 【放大镜测试】【无效输入】非有限坐标与无效像素比例不能构造或改变草稿；无效倍率回到原版默认值
#[test]
fn non_finite_input_cannot_create_invalid_geometry() {
    let bounds = Rect::from_xywh(0.0, 0.0, 400.0, 300.0).unwrap();
    for scale in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        assert!(
            Magnifier::new(MagnifierShape::Circle, (10.0, 10.0), 2.75, bounds, scale).is_none()
        );
    }
    assert!(Magnifier::new(MagnifierShape::Circle, (f32::NAN, 10.0), 2.75, bounds, 1.0).is_none());
    assert!(Magnifier::new(MagnifierShape::Circle, (-1.0, 10.0), 2.75, bounds, 1.0).is_none());
    let magnifier = draft(MagnifierShape::Rectangle, 1.0).draft_to((240.0, 220.0));
    assert_eq!(magnifier.draft_to((f32::NAN, 10.0)), magnifier);
    near(magnifier.with_zoom(f32::INFINITY).zoom, 2.75);
}

/// 【放大镜测试】【导出坐标】导出映射改变几何与绘制单位，不改变放大倍率或原图取样尺寸
#[test]
fn export_mapping_preserves_original_sample_pixels_and_style() {
    let before = draft(MagnifierShape::Rectangle, 1.6).draft_to((240.0, 250.0));
    let after = before.scaled(1.6);
    near(before.zoom, after.zoom);
    near(
        before.source_rect().unwrap().width() * before.pixel_scale,
        after.source_rect().unwrap().width() * after.pixel_scale,
    );
    near(before.border_width(40.0) * 1.6, after.border_width(64.0));
    near(after.source.0, before.source.0 * 1.6);
    near(after.pivot.1, before.pivot.1 * 1.6);
    assert_eq!(before, before.scaled(f32::NAN));
}

/// 【放大镜测试】【独立形状】矩形转换为圆形时使用短边，不改变两个框的中心或放大倍率
#[test]
fn circle_conversion_keeps_centers_and_normalizes_the_frame() {
    let mut before = draft(MagnifierShape::Rectangle, 1.0).draft_to((300.0, 240.0));
    before.size = (180.0, 100.0);
    let after = before.with_shape(MagnifierShape::Circle);
    assert_eq!(after.size, (100.0, 100.0));
    assert_eq!(after.center, before.center);
    assert_eq!(after.source, before.source);
    near(after.zoom, before.zoom);
}

/// 【放大镜测试】【连接线】接近重叠时不画连接线，远离时两条连接线保留原版三十四度取点
#[test]
fn connectors_are_suppressed_for_overlapping_frames() {
    let far = draft(MagnifierShape::Circle, 1.0).draft_to((300.0, 180.0));
    let lines = far.connectors().unwrap();
    assert!(lines[0].0.1 > far.source.1);
    assert!(lines[1].0.1 < far.source.1);
    let mut overlap = far;
    overlap.center = overlap.source;
    assert!(overlap.connectors().is_none());
}

/// 【放大镜测试】【旋转边界】旋转后的正文边界包含双框角点，旋转前后取样区域不发生变化
#[test]
fn rotation_bounds_cover_both_frames_without_resampling_a_different_region() {
    let mut magnifier = draft(MagnifierShape::Rectangle, 1.0).draft_to((300.0, 240.0));
    let source = magnifier.source_rect();
    magnifier.rotation = std::f32::consts::FRAC_PI_4;
    let bounds = magnifier.bounds().unwrap().outset(0.001, 0.001).unwrap();
    for rect in [
        magnifier.lens_rect().unwrap(),
        magnifier.source_rect().unwrap(),
    ] {
        for point in [
            (rect.left(), rect.top()),
            (rect.right(), rect.top()),
            (rect.right(), rect.bottom()),
            (rect.left(), rect.bottom()),
        ] {
            let point = magnifier.to_visual(point);
            assert!(point.0 >= bounds.left() && point.0 <= bounds.right());
            assert!(point.1 >= bounds.top() && point.1 <= bounds.bottom());
        }
    }
    assert_eq!(source, magnifier.source_rect());
}
