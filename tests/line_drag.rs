//! 【线条测试】【定点与变换】验证外部控制点、端点、全骨架移动、缩放、旋转和完整编辑损坏范围

use mark_shot::features::line::{LineSkeleton, LineStyle, drag, draw, polyline::distance};
use mark_shot::types::SelectionHandle;
use tiny_skia::Rect;

/// 【线条测试】【固定骨架】返回两个端点及一个外部控制点的骨架和实际正文边界
fn fixture() -> (LineSkeleton, Rect) {
    let mut line = LineSkeleton::new((100.0, 120.0), (300.0, 120.0)).unwrap();
    line.controls.push((200.0, 300.0));
    let bounds = draw::outline(&line, LineStyle::Solid, 3.0)
        .unwrap()
        .bounds();
    (line, bounds)
}

/// 【线条测试】【画布边界】返回不连接桌面的六百四十乘四百八十逻辑画布
fn canvas() -> Rect {
    Rect::from_xywh(0.0, 0.0, 640.0, 480.0).unwrap()
}

/// 【线条测试】【外部定点】已选控制点即使位于正文外仍能命中，未选择时不可点击不可见控制点
#[test]
fn selected_control_points_are_hit_outside_the_curve_body() {
    let (line, bounds) = fixture();
    assert!(line.controls[0].1 > bounds.bottom());
    assert_eq!(
        drag::hit_test(&line, bounds, 3.0, line.controls[0], true),
        SelectionHandle::LinePoint(2)
    );
    assert_eq!(
        drag::hit_test(&line, bounds, 3.0, line.controls[0], false),
        SelectionHandle::None
    );
    assert_eq!(
        drag::hit_test(&line, bounds, 3.0, line.start, false),
        SelectionHandle::LinePoint(0)
    );
    assert_eq!(
        drag::hit_test(&line, bounds, 3.0, line.end, false),
        SelectionHandle::LinePoint(1)
    );
}

/// 【线条测试】【外框空间】细直线的端点不遮挡外框缩放，旋转把手在正文之外优先命中
#[test]
fn thin_lines_have_independent_frame_and_rotation_handles() {
    let line = LineSkeleton::new((100.0, 120.0), (300.0, 120.0)).unwrap();
    let bounds = draw::outline(&line, LineStyle::Solid, 3.0)
        .unwrap()
        .bounds();
    let frame = drag::frame(bounds).unwrap();
    assert_eq!(
        drag::hit_test(&line, bounds, 3.0, (frame.right(), frame.top()), true),
        SelectionHandle::TopRight
    );
    assert_eq!(
        drag::hit_test(
            &line,
            bounds,
            3.0,
            drag::rotation_point(&line, bounds),
            true
        ),
        SelectionHandle::Rotate
    );
}

/// 【线条测试】【旋转定点分离】上方曲线控制点不得遮挡旋转把手，两类把手必须能各自命中
#[test]
fn rotation_handle_stays_separate_from_an_external_curve_control() {
    let mut line = LineSkeleton::new((215.0, 535.0), (450.0, 535.0)).unwrap();
    line.controls.push((330.0, 455.0));
    let bounds = mark_shot::features::arrow::path::outline(
        &line,
        mark_shot::features::arrow::ArrowStyle::Fletched,
        LineStyle::Solid,
        3.0,
    )
    .unwrap()
    .bounds();
    let rotation = drag::rotation_point(&line, bounds);
    assert_eq!(
        drag::hit_test(&line, bounds, 3.0, rotation, true),
        SelectionHandle::Rotate
    );
    assert_eq!(
        drag::hit_test(&line, bounds, 3.0, line.controls[0], true),
        SelectionHandle::LinePoint(2)
    );
    assert!(distance(rotation, line.controls[0]) > 20.0);
}

/// 【线条测试】【独立定点】拖动控制点或端点只改变指定索引，画布外位置按原图范围约束
#[test]
fn endpoint_and_control_drags_preserve_other_points() {
    let (line, bounds) = fixture();
    let moved = drag::dragged(
        &line,
        bounds,
        SelectionHandle::LinePoint(2),
        line.controls[0],
        (220.0, 340.0),
        canvas(),
        false,
        false,
    );
    assert_eq!(moved.start, line.start);
    assert_eq!(moved.end, line.end);
    assert_eq!(moved.controls[0], (220.0, 340.0));
    let moved = drag::dragged(
        &line,
        bounds,
        SelectionHandle::LinePoint(1),
        line.end,
        (900.0, -20.0),
        canvas(),
        false,
        false,
    );
    assert_eq!(moved.start, line.start);
    assert_eq!(moved.controls, line.controls);
    assert_eq!(moved.end, (640.0, 0.0));
}

/// 【线条测试】【整体移动】移动以原始快照为基准，并保留全部端点和控制点的相对位置
#[test]
fn moving_a_skeleton_preserves_all_relative_geometry() {
    let (line, bounds) = fixture();
    let moved = drag::dragged(
        &line,
        bounds,
        SelectionHandle::Move,
        (200.0, 180.0),
        (240.0, 200.0),
        canvas(),
        false,
        false,
    );
    for (before, after) in line.points().zip(moved.points()) {
        assert_eq!(after, (before.0 + 40.0, before.1 + 20.0));
    }
    assert_eq!(moved.style_scale, line.style_scale);
}

/// 【线条测试】【等比缩放】Ctrl 外框缩放使用同一比例映射骨架，重复相同输入没有累计误差
#[test]
fn control_resize_preserves_shape_ratio_and_original_snapshot() {
    let (line, bounds) = fixture();
    let frame = drag::frame(bounds).unwrap();
    let start = (frame.right(), frame.top());
    let end = (start.0 + 80.0, start.1 - 45.0);
    let moved = drag::dragged(
        &line,
        bounds,
        SelectionHandle::TopRight,
        start,
        end,
        canvas(),
        true,
        false,
    );
    let x_ratio = (moved.end.0 - moved.start.0) / (line.end.0 - line.start.0);
    let y_ratio = (moved.controls[0].1 - moved.start.1) / (line.controls[0].1 - line.start.1);
    assert!((x_ratio - y_ratio).abs() < 0.0001);
    assert_eq!(
        moved,
        drag::dragged(
            &line,
            bounds,
            SelectionHandle::TopRight,
            start,
            end,
            canvas(),
            true,
            false
        )
    );
}

/// 【线条测试】【旋转吸附】Shift 旋转吸附到四十五度，端点和控制点共用中心与角度
#[test]
fn shift_rotation_snaps_bearing_and_preserves_distances() {
    let (line, bounds) = fixture();
    let center = (
        (bounds.left() + bounds.right()) * 0.5,
        (bounds.top() + bounds.bottom()) * 0.5,
    );
    let start = drag::rotation_point(&line, bounds);
    let angle = 32.0_f32.to_radians();
    let delta = (start.0 - center.0, start.1 - center.1);
    let end = (
        center.0 + delta.0 * angle.cos() - delta.1 * angle.sin(),
        center.1 + delta.0 * angle.sin() + delta.1 * angle.cos(),
    );
    let moved = drag::dragged(
        &line,
        bounds,
        SelectionHandle::Rotate,
        start,
        end,
        canvas(),
        false,
        true,
    );
    let bearing = (moved.end.1 - moved.start.1)
        .atan2(moved.end.0 - moved.start.0)
        .to_degrees();
    assert!((bearing - 45.0).abs() < 0.001);
    assert!(
        (distance(moved.start, moved.controls[0]) - distance(line.start, line.controls[0])).abs()
            < 0.001
    );
}

/// 【线条测试】【编辑范围】完整编辑损坏区域包含实际曲线外的控制点及旋转把手，避免残留像素
#[test]
fn editing_damage_contains_external_control_points_and_rotation() {
    let (line, bounds) = fixture();
    let editing = drag::editing_bounds(&line, bounds, 5.0).unwrap();
    let control = line.controls[0];
    let rotation = drag::rotation_point(&line, bounds);
    assert!(editing.bottom() >= control.1 + 9.0);
    assert!(editing.top() <= rotation.1 - 9.0);
    assert!(editing.left() < bounds.left() && editing.right() > bounds.right());
}
