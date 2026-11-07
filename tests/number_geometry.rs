//! 【序号测试】【几何边界】验证圆框、尾部、旋转轴心和原图比例

use mark_shot::features::number::{NumberMark, NumberStyle, WIDTH, clamped_width};
use mark_shot::types::{Annotation, AnnotationShape};
use tiny_skia::{Color, Rect};

/// 【序号测试】【标记构造】返回圆心固定、带右向尖端的序号；style 指定标签格式
fn mark(style: NumberStyle) -> NumberMark {
    let mut mark = NumberMark::new((100.0, 120.0), 27, style).unwrap();
    mark.tip = (240.0, 120.0);
    mark
}

/// 【序号测试】【近似比较】验证 actual 和 expected 的误差小于万分之一
fn near(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.0001, "{actual} != {expected}");
}

/// 【序号测试】【默认尺寸】圆框半径、白色边框和尾部尺寸遵循原版公式
#[test]
fn bubble_and_outline_sizes_match_the_reference() {
    let mark = mark(NumberStyle::Arabic);
    near(mark.bubble_radius(3.0), 17.05);
    near(mark.outline_width(3.0), 2.0);
    near(mark.outline_width(72.0), 9.0);
    assert_eq!(WIDTH, (1.0, 72.0));
    assert_eq!(clamped_width(f32::NAN, 1.0), 3.0);
    assert_eq!(clamped_width(f32::INFINITY, 1.0), 3.0);
    assert_eq!(clamped_width(1000.0, 1.0), 72.0);
}

/// 【序号测试】【边界包含】正文边界包含白色边框和指向尖端，不依赖字体加载
#[test]
fn bounds_include_the_full_ring_and_leader_tip() {
    let mark = mark(NumberStyle::Chinese);
    let bounds = mark.body_bounds(3.0).unwrap();
    near(bounds.left(), 81.95);
    near(bounds.top(), 101.95);
    assert!(bounds.right() >= 240.0);
    assert!(bounds.bottom() >= 138.05);
    let tail = mark.tail(3.0).unwrap();
    assert_eq!(tail[0], mark.tip);
    for point in tail {
        assert!(point.0 >= bounds.left() && point.0 <= bounds.right());
        assert!(point.1 >= bounds.top() && point.1 <= bounds.bottom());
    }
}

/// 【序号测试】【单击标记】尖端与圆心重合时只创建圆框，不生成零长度尾部
#[test]
fn a_click_is_a_valid_standalone_bubble() {
    let mark = NumberMark::new((50.0, 60.0), 1, NumberStyle::Arabic).unwrap();
    assert!(mark.tail(3.0).is_none());
    assert!(mark.body_bounds(3.0).is_some());
}

/// 【序号测试】【旋转轴心】旋转以圆心为轴，保持圆心和圆框范围，尖端按角度改变
#[test]
fn rotation_uses_the_bubble_as_its_pivot() {
    let mut mark = mark(NumberStyle::UpperRoman);
    mark.rotation = std::f32::consts::FRAC_PI_2;
    let tip = mark.to_visual(mark.tip);
    near(tip.0, 100.0);
    near(tip.1, 260.0);
    let local = mark.to_local(tip);
    near(local.0, 240.0);
    near(local.1, 120.0);
    let bounds = mark.body_bounds(3.0).unwrap();
    near(bounds.left(), 81.95);
    near(bounds.bottom(), 260.0);
}

/// 【序号测试】【导出比例】导出同时缩放固定圆框留量与笔宽，标签、编号和旋转角度保持不变
#[test]
fn export_scaling_is_homogeneous_and_preserves_label_semantics() {
    let mark = mark(NumberStyle::LowerAlpha);
    let scaled = mark.scaled(2.5);
    assert_eq!(scaled.style, mark.style);
    assert_eq!(scaled.number, mark.number);
    assert_eq!(scaled.bubble, (250.0, 300.0));
    assert_eq!(scaled.tip, (600.0, 300.0));
    near(scaled.bubble_radius(7.5), mark.bubble_radius(3.0) * 2.5);
    near(scaled.outline_width(7.5), mark.outline_width(3.0) * 2.5);
    let before = mark.body_bounds(3.0).unwrap();
    let after = scaled.body_bounds(7.5).unwrap();
    near(after.left(), before.left() * 2.5);
    near(after.right(), before.right() * 2.5);
}

/// 【序号测试】【标注映射】统一标注接口保留序号样式，支持导出缩放与整体平移
#[test]
fn annotation_transforms_preserve_the_number_and_style() {
    let mut annotation = Annotation {
        style_scale: 1.0,
        rotation_degrees: 0.0,
        frozen_bounds: None,
        id: 9,
        shape: AnnotationShape::Number {
            mark: mark(NumberStyle::Chinese),
        },
        color: Color::from_rgba8(20, 100, 200, 128),
        shadow_color: Color::TRANSPARENT,
        stroke_width: 3.0,
        bbox: Rect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap(),
    };
    annotation.update_bbox();
    let scaled = annotation.scaled(2.0);
    annotation.translate_mut(-40.0, 30.0);
    let AnnotationShape::Number { mark } = annotation.shape else {
        panic!("number expected")
    };
    assert_eq!(mark.bubble, (60.0, 150.0));
    assert_eq!(mark.tip, (200.0, 150.0));
    let AnnotationShape::Number { mark } = scaled.shape else {
        panic!("number expected")
    };
    assert_eq!(mark.bubble, (200.0, 240.0));
    assert_eq!(mark.style, NumberStyle::Chinese);
    assert_eq!(mark.number, 27);
}

/// 【序号测试】【异常输入】非有限坐标、无效编号和无效比例不产生可绘制几何
#[test]
fn invalid_geometry_never_panics_or_allocates_a_label() {
    assert!(NumberMark::new((f32::NAN, 0.0), 1, NumberStyle::Arabic).is_none());
    assert!(NumberMark::new((0.0, f32::INFINITY), 1, NumberStyle::Arabic).is_none());
    assert!(NumberMark::new((0.0, 0.0), 0, NumberStyle::Arabic).is_none());
    let mut mark = mark(NumberStyle::Arabic);
    mark.rotation = f32::NAN;
    assert!(mark.body_bounds(3.0).is_none());
    mark.rotation = 0.0;
    mark.style_scale = 0.0;
    assert!(mark.body_bounds(3.0).is_none());
    mark.style_scale = -1.0;
    assert!(mark.body_bounds(3.0).is_none());
    assert_eq!(mark.outline_width(3.0), 2.0);
}
