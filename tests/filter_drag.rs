//! 【滤镜测试】【旋转拖动】验证视觉锚点、最小尺寸、旋转吸附和源图约束

use mark_shot::features::filter::{FilterMode, RegionFilter, drag};
use mark_shot::types::SelectionHandle;
use tiny_skia::Rect;

/// 【滤镜测试】【区域构造】返回指定 rotation 的反色区域和宽松原图范围
fn region(rotation: f32) -> (RegionFilter, Rect) {
    let mut filter =
        RegionFilter::new((200.0, 180.0), (380.0, 280.0), 14.0, FilterMode::Invert).unwrap();
    filter.rotation = rotation;
    (filter, Rect::from_xywh(0.0, 0.0, 1000.0, 800.0).unwrap())
}

/// 【滤镜测试】【控制点命中】八个旋转控制点与把手都命中正确操作，未选中时外框不接受初次点击
#[test]
fn rotated_handles_follow_the_visible_frame() {
    let (filter, _) = region(0.37);
    for (handle, point) in drag::controls(filter) {
        assert_eq!(drag::hit_test(filter, point, true), handle);
    }
    let handle = drag::rotation_handle(filter).unwrap();
    assert_eq!(
        drag::hit_test(filter, handle, true),
        SelectionHandle::Rotate
    );
    assert_eq!(drag::hit_test(filter, handle, false), SelectionHandle::None);
    assert_eq!(
        drag::hit_test(filter, filter.center(), false),
        SelectionHandle::Move
    );
    assert_eq!(
        drag::hit_test(filter, (f32::NAN, 0.0), true),
        SelectionHandle::None
    );
}

/// 【滤镜测试】【旋转吸附】Shift 把三十度旋转吸附为四十五度，强度、源框和模式保持不变
#[test]
fn shift_rotation_snaps_to_reference_45_degree_steps() {
    let (filter, bounds) = region(0.0);
    let start = drag::rotation_handle(filter).unwrap();
    let center = filter.center();
    let delta = (start.0 - center.0, start.1 - center.1);
    let angle = std::f32::consts::PI / 6.0;
    let point = (
        center.0 + delta.0 * angle.cos() - delta.1 * angle.sin(),
        center.1 + delta.0 * angle.sin() + delta.1 * angle.cos(),
    );
    let free = drag::dragged(
        filter,
        SelectionHandle::Rotate,
        start,
        point,
        bounds,
        false,
        false,
    );
    let snapped = drag::dragged(
        filter,
        SelectionHandle::Rotate,
        start,
        point,
        bounds,
        false,
        true,
    );
    assert!((free.rotation - angle).abs() < 0.0001);
    assert!((snapped.rotation - std::f32::consts::FRAC_PI_4).abs() < 0.0001);
    assert_eq!(snapped.rect(), filter.rect());
    assert_eq!(snapped.strength, 14.0);
    assert_eq!(snapped.mode, FilterMode::Invert);
}

/// 【滤镜测试】【旋转缩放锚点】旋转后拉动右下角，对侧左上角在屏幕中保持不动
#[test]
fn rotated_resize_keeps_the_opposite_visual_corner_fixed() {
    let (filter, bounds) = region(0.37);
    let rect = filter.rect().unwrap();
    let anchored = filter.to_visual((rect.left(), rect.top()));
    let start = drag::controls(filter)
        .into_iter()
        .find(|(handle, _)| *handle == SelectionHandle::BottomRight)
        .unwrap()
        .1;
    let point = filter.to_visual((rect.right() + 36.0 + 6.0, rect.bottom() + 20.0 + 6.0));
    let resized = drag::dragged(
        filter,
        SelectionHandle::BottomRight,
        start,
        point,
        bounds,
        false,
        false,
    );
    let rect = resized.rect().unwrap();
    let actual = resized.to_visual((rect.left(), rect.top()));
    assert!((actual.0 - anchored.0).abs() < 0.0001);
    assert!((actual.1 - anchored.1).abs() < 0.0001);
    assert!((rect.width() - 216.0).abs() < 0.0001);
    assert!((rect.height() - 120.0).abs() < 0.0001);
    assert_eq!(resized.strength, filter.strength);
}

/// 【滤镜测试】【等比缩放】Ctrl 在侧边和角点缩放时锁定源框比例，不缩放滤镜强度
#[test]
fn ctrl_resize_keeps_aspect_ratio_without_scaling_strength() {
    let (filter, bounds) = region(0.0);
    for handle in [SelectionHandle::Right, SelectionHandle::BottomRight] {
        let start = drag::controls(filter)
            .into_iter()
            .find(|(candidate, _)| *candidate == handle)
            .unwrap()
            .1;
        let resized = drag::dragged(
            filter,
            handle,
            start,
            (start.0 + 90.0, start.1 + 10.0),
            bounds,
            true,
            false,
        );
        let rect = resized.rect().unwrap();
        assert!((rect.width() / rect.height() - 1.8).abs() < 0.0001);
        assert_eq!(resized.strength, filter.strength);
    }
}

/// 【滤镜测试】【窄区域最小尺寸】等比缩小仍保证两条边至少两个原图像素，避免退化滤镜
#[test]
fn aspect_locked_thin_regions_keep_both_minimum_dimensions() {
    let filter = RegionFilter::new((100.0, 100.0), (300.0, 104.0), 14.0, FilterMode::Blur).unwrap();
    let start = drag::controls(filter)
        .into_iter()
        .find(|(handle, _)| *handle == SelectionHandle::Right)
        .unwrap()
        .1;
    let resized = drag::dragged(
        filter,
        SelectionHandle::Right,
        start,
        (start.0 - 199.0, start.1),
        Rect::from_xywh(0.0, 0.0, 1000.0, 800.0).unwrap(),
        true,
        false,
    );
    let rect = resized.rect().unwrap();
    assert!(rect.width() >= 2.0 && rect.height() >= 2.0);
    assert!((rect.width() / rect.height() - 50.0).abs() < 0.001);
}

/// 【滤镜测试】【平移边界】移动不能把源框移出原图，过大的外部区域也不因反转范围而异常
#[test]
fn moves_are_bounded_and_oversized_external_rectangles_do_not_panic() {
    let (filter, bounds) = region(0.37);
    let moved = drag::dragged(
        filter,
        SelectionHandle::Move,
        filter.center(),
        (-1000.0, -1000.0),
        bounds,
        false,
        false,
    );
    let rect = moved.rect().unwrap();
    assert!(rect.left() >= bounds.left() && rect.top() >= bounds.top());
    let oversized =
        RegionFilter::new((-500.0, -500.0), (2000.0, 1600.0), 14.0, FilterMode::Blur).unwrap();
    let moved = drag::dragged(
        oversized,
        SelectionHandle::Move,
        oversized.center(),
        (0.0, 0.0),
        bounds,
        false,
        false,
    );
    assert!(moved.valid());
}
