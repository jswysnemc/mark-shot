//! 【放大镜测试】【双框交互】验证控制点优先级、独立移动、联动缩放和旋转

use mark_shot::features::magnifier::drag;
use mark_shot::features::magnifier::{Magnifier, MagnifierShape};
use mark_shot::types::SelectionHandle;
use tiny_skia::Rect;

/// 【拖动测试】【双框构造】返回 shape 指定的分离双框，使用原图像素比例 1.5
fn magnifier(shape: MagnifierShape) -> Magnifier {
    Magnifier::new(
        shape,
        (120.0, 220.0),
        2.75,
        Rect::from_xywh(0.0, 0.0, 1000.0, 600.0).unwrap(),
        1.5,
    )
    .unwrap()
    .draft_to((320.0, 220.0))
}

/// 【拖动测试】【双框比例】验证 value 的两框保持固定倍率，且未旋转几何完整处于原图内
fn valid(value: Magnifier) {
    let lens = value.lens_rect().unwrap();
    let source = value.source_rect().unwrap();
    assert!((lens.width() / source.width() - value.zoom).abs() < 0.001);
    assert!((lens.height() / source.height() - value.zoom).abs() < 0.001);
    for frame in [lens, source] {
        assert!(frame.left() >= value.image_bounds.left() - 0.001);
        assert!(frame.top() >= value.image_bounds.top() - 0.001);
        assert!(frame.right() <= value.image_bounds.right() + 0.001);
        assert!(frame.bottom() <= value.image_bounds.bottom() + 0.001);
    }
}

/// 【拖动测试】【控制点数量】矩形双框各有八点，圆形双框各有四点，旋转把手独立于两组控制点
#[test]
fn each_frame_has_shape_specific_controls_and_an_independent_rotation_handle() {
    for (shape, count) in [(MagnifierShape::Circle, 8), (MagnifierShape::Rectangle, 16)] {
        let value = magnifier(shape);
        assert_eq!(drag::controls(value).len(), count);
        for (handle, point) in drag::controls(value) {
            assert_eq!(drag::hit_test(value, point, true), handle);
        }
        assert_eq!(
            drag::hit_test(value, drag::rotation_handle(value).unwrap(), true),
            SelectionHandle::Rotate
        );
    }
}

/// 【拖动测试】【重叠优先级】两框重叠时镜框控制点优先，内部区域优先选择源框，不命中两个框之间的空白
#[test]
fn overlapping_frames_prioritize_lens_controls_then_source_interiors() {
    let mut value = magnifier(MagnifierShape::Rectangle);
    value.center = value.source;
    assert_eq!(
        drag::hit_test(value, value.source, true),
        SelectionHandle::MagnifierSource
    );
    let lens = value.lens_rect().unwrap();
    assert_eq!(
        drag::hit_test(value, (lens.left(), lens.top()), true),
        SelectionHandle::TopLeft
    );
    let separated = magnifier(MagnifierShape::Circle);
    assert_eq!(
        drag::hit_test(separated, (220.0, 100.0), false),
        SelectionHandle::None
    );
}

/// 【拖动测试】【独立移动】源框或镜框平移不改变另一框中心、尺寸与倍率；超出范围时约束完整框
#[test]
fn moving_one_frame_does_not_move_the_other_or_change_zoom() {
    for shape in MagnifierShape::ALL {
        let before = magnifier(shape);
        let source = drag::dragged(
            before,
            SelectionHandle::MagnifierSource,
            before.source,
            (180.0, 260.0),
            false,
            false,
        );
        assert_eq!(source.center, before.center);
        assert_eq!(source.size, before.size);
        assert_ne!(source.source, before.source);
        valid(source);
        let lens = drag::dragged(
            before,
            SelectionHandle::MagnifierLens,
            before.center,
            (1200.0, -500.0),
            false,
            false,
        );
        assert_eq!(lens.source, before.source);
        assert_eq!(lens.size, before.size);
        valid(lens);
    }
}

/// 【拖动测试】【源框联动】源框任一控制点缩放后镜框按固定倍率联动，镜框中心保持不变
#[test]
fn all_source_resize_handles_scale_the_lens_at_a_fixed_center() {
    for shape in MagnifierShape::ALL {
        let before = magnifier(shape);
        for (handle, point) in drag::controls(before)
            .into_iter()
            .filter(|(handle, _)| drag::is_source_resize(*handle))
        {
            let after = drag::dragged(
                before,
                handle,
                point,
                (point.0 + 30.0, point.1 + 25.0),
                false,
                false,
            );
            valid(after);
            assert_eq!(after.center, before.center);
            if shape == MagnifierShape::Circle {
                assert_eq!(after.size.0, after.size.1);
            }
        }
    }
}

/// 【拖动测试】【镜框联动】镜框任一控制点缩放后源框按固定倍率联动，远离边缘的源框中心保持不变
#[test]
fn all_lens_resize_handles_scale_the_source_at_a_fixed_center() {
    for shape in MagnifierShape::ALL {
        let before = magnifier(shape);
        for (handle, point) in drag::controls(before)
            .into_iter()
            .filter(|(handle, _)| !drag::is_source_resize(*handle))
        {
            let after = drag::dragged(
                before,
                handle,
                point,
                (point.0 + 30.0, point.1 + 25.0),
                false,
                false,
            );
            valid(after);
            assert_eq!(after.source, before.source);
            if shape == MagnifierShape::Circle {
                assert_eq!(after.size.0, after.size.1);
            }
        }
    }
}

/// 【拖动测试】【范围与极值】极远控制点、交叉拖动与两组控制点均不能破坏倍率或产生负尺寸
#[test]
fn extreme_and_crossed_resizes_keep_full_frames_at_the_same_zoom() {
    for shape in MagnifierShape::ALL {
        let before = magnifier(shape);
        for (handle, point) in drag::controls(before) {
            for target in [(-10000.0, -10000.0), (10000.0, 10000.0), before.source] {
                valid(drag::dragged(before, handle, point, target, false, false));
            }
        }
    }
}

/// 【拖动测试】【矩形等比】Ctrl 调整矩形框保留原有长宽比，不强制转成正方形
#[test]
fn control_preserves_a_rectangular_frames_aspect_ratio() {
    let mut before = magnifier(MagnifierShape::Rectangle);
    before.size = (180.0, 100.0);
    let rect = before.lens_rect().unwrap();
    let point = (rect.right(), rect.bottom());
    let after = drag::dragged(
        before,
        SelectionHandle::BottomRight,
        point,
        (point.0 + 80.0, point.1 + 10.0),
        true,
        false,
    );
    assert!((after.size.0 / after.size.1 - 1.8).abs() < 0.001);
    valid(after);
}

/// 【拖动测试】【旋转吸附】Shift 将整组双框旋转吸附到四十五度，不修改原图取样点或倍率
#[test]
fn shift_rotation_snaps_the_dual_frame_without_changing_the_sample() {
    let before = magnifier(MagnifierShape::Rectangle);
    let handle = drag::rotation_handle(before).unwrap();
    let vector = (handle.0 - before.pivot.0, handle.1 - before.pivot.1);
    let radians = 34.0_f32.to_radians();
    let (sin, cos) = radians.sin_cos();
    let point = (
        before.pivot.0 + vector.0 * cos - vector.1 * sin,
        before.pivot.1 + vector.0 * sin + vector.1 * cos,
    );
    let after = drag::dragged(before, SelectionHandle::Rotate, handle, point, false, true);
    assert!((after.rotation - std::f32::consts::FRAC_PI_4).abs() < 0.001);
    assert_eq!(after.source, before.source);
    assert_eq!(after.center, before.center);
    assert_eq!(after.zoom, before.zoom);
}

/// 【拖动测试】【旋转后移动】旋转后的独立平移跟随屏幕方向，另一框的视觉位置保持不变
#[test]
fn rotated_independent_movement_tracks_the_pointer_without_moving_the_other_frame() {
    let mut before = magnifier(MagnifierShape::Rectangle);
    before.rotation = std::f32::consts::FRAC_PI_4;
    let start = before.to_visual(before.center);
    let after = drag::dragged(
        before,
        SelectionHandle::MagnifierLens,
        start,
        (start.0 + 20.0, start.1 + 10.0),
        false,
        false,
    );
    let center = after.to_visual(after.center);
    assert!((center.0 - start.0 - 20.0).abs() < 0.001);
    assert!((center.1 - start.1 - 10.0).abs() < 0.001);
    assert_eq!(
        before.to_visual(before.source),
        after.to_visual(after.source)
    );
}
