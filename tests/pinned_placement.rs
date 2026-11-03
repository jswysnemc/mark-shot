//! 【贴图测试】【输出放置】负坐标、输出间隙、裁切偏移和最小可见区域

use mark_shot::features::pinned_image::placement::{Geometry, best_output, place};

/// 【贴图测试】【相交选择】使用相交面积而非指针所在输出决定归属，面积相同时以中心距离决胜
#[test]
fn output_is_selected_by_visible_area_then_distance() {
    let screens = [
        Geometry::new((-800, 0), (800, 600)),
        Geometry::new((0, 0), (1000, 800)),
    ];
    assert_eq!(
        best_output(Geometry::new((-100, 150), (240, 120)), &screens),
        Some(1)
    );
    assert_eq!(
        best_output(Geometry::new((-160, 150), (240, 120)), &screens),
        Some(0)
    );
    assert_eq!(
        best_output(Geometry::new((-120, 150), (240, 120)), &screens),
        Some(0)
    );
    assert_eq!(
        best_output(Geometry::new((1500, 0), (240, 120)), &screens),
        Some(1)
    );
}

/// 【贴图测试】【可见裁切】左上越界只裁切表面，不压缩原图，绘制偏移等于完整原点减可见原点
#[test]
fn clipping_keeps_logical_size_and_source_origin() {
    let screen = Geometry::new((-800, 50), (800, 600));
    let result = place(Geometry::new((-850, 20), (240, 120)), screen);
    assert_eq!(result.logical, Geometry::new((-850, 20), (240, 120)));
    assert_eq!(result.visible, Geometry::new((-800, 50), (190, 90)));
    assert_eq!(result.offset, (-50, -30));
    assert_eq!(result.margin, (0, 0));
}

/// 【贴图测试】【保留可见】完整拖出屏幕后保留二十四像素可见，并适配窄图或窄输出
#[test]
fn complete_offscreen_drag_retains_a_recoverable_strip() {
    let screen = Geometry::new((0, 0), (800, 600));
    let left = place(Geometry::new((-2000, -2000), (240, 120)), screen);
    assert_eq!(left.logical, Geometry::new((-216, -96), (240, 120)));
    assert_eq!(left.visible, Geometry::new((0, 0), (24, 24)));
    let right = place(Geometry::new((2000, 2000), (240, 120)), screen);
    assert_eq!(right.visible, Geometry::new((776, 576), (24, 24)));
    let small = place(
        Geometry::new((100, 100), (8, 10)),
        Geometry::new((0, 0), (6, 5)),
    );
    assert_eq!(small.visible, Geometry::new((0, 0), (6, 5)));
}

/// 【贴图测试】【超大逻辑图】放大后的完整几何超过显示器仍只提交可见尺寸，不改变显示比例
#[test]
fn huge_zoom_uses_only_the_output_sized_surface() {
    let result = place(
        Geometry::new((-3000, -2000), (12000, 6000)),
        Geometry::new((0, 0), (1280, 800)),
    );
    assert_eq!(result.logical.width, 12000);
    assert_eq!(result.visible, Geometry::new((0, 0), (1280, 800)));
    assert_eq!(result.offset, (-3000, -2000));
}

/// 【贴图测试】【非法输出】空输出、零尺寸矩形和极端坐标不溢出，不选择无效屏幕
#[test]
fn invalid_screens_and_extreme_coordinates_are_safe() {
    let image = Geometry::new((i32::MAX - 10, i32::MIN), (240, 120));
    assert_eq!(image.right(), i64::from(i32::MAX) + 230);
    assert_eq!(best_output(image, &[]), None);
    assert_eq!(best_output(image, &[Geometry::default()]), None);
    let result = place(image, Geometry::new((i32::MIN, i32::MIN), (800, 600)));
    assert_eq!(result.visible.width, 24);
    assert_eq!(place(image, Geometry::default()).visible.width, 0);
}
