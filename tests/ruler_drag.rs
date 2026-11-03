//! 【尺子测试】【控制点】验证长度框内部命中、角度定点、旋转与范围调整

use mark_shot::features::ruler::drag::{self, ANGLE_FRAME_PADDING};
use mark_shot::features::ruler::geometry::distance;
use mark_shot::features::ruler::{Measurement, RulerGeometry, RulerMode};
use mark_shot::types::SelectionHandle;

/// 【尺子测试】【长度构造】返回可旋转的 100×80 逻辑单位长度尺
fn length(rotation: f32) -> Measurement {
    let mut ruler = Measurement::new(RulerMode::Length, (100.0, 100.0), 1.5).unwrap();
    ruler.geometry = RulerGeometry::Length {
        start: (100.0, 100.0),
        end: (200.0, 180.0),
        rotation,
    };
    ruler
}

/// 【尺子测试】【角度构造】返回第一边向右、第二边斜向下的完整角度尺
fn angle() -> Measurement {
    let mut ruler = Measurement::new(RulerMode::Angle, (100.0, 100.0), 1.5).unwrap();
    ruler.geometry = RulerGeometry::Angle {
        points: [(100.0, 100.0), (200.0, 100.0), (170.0, 180.0)],
        count: 3,
    };
    ruler
}

/// 【尺子测试】【内部留空】长度框内部在尺子工具中留给新测量，选择工具可以整体移动
#[test]
fn length_interior_is_reserved_for_new_measurements() {
    let ruler = length(0.0);
    assert_eq!(
        drag::hit_test(ruler, (150.0, 140.0), 3.0, true, false),
        SelectionHandle::None
    );
    assert_eq!(
        drag::hit_test(ruler, (150.0, 140.0), 3.0, true, true),
        SelectionHandle::Move
    );
    assert_eq!(
        drag::hit_test(ruler, (150.0, 101.0), 3.0, false, false),
        SelectionHandle::Top
    );
    let ruler = ruler.rotated(90.0);
    assert_eq!(
        drag::hit_test(ruler, (150.0, 140.0), 3.0, true, false),
        SelectionHandle::None
    );
}

/// 【尺子测试】【角度定点】三个定点分别命中；只移动顶点不整体平移另外两点
#[test]
fn angle_vertex_and_arm_handles_are_independent() {
    let ruler = angle();
    assert_eq!(
        drag::hit_test(ruler, (100.0, 100.0), 3.0, false, false),
        SelectionHandle::AngleVertex
    );
    assert_eq!(
        drag::hit_test(ruler, (200.0, 100.0), 3.0, false, false),
        SelectionHandle::AngleFirstArm
    );
    assert_eq!(
        drag::hit_test(ruler, (170.0, 180.0), 3.0, false, false),
        SelectionHandle::AngleSecondArm
    );
    let moved = drag::dragged(
        ruler,
        SelectionHandle::AngleVertex,
        (100.0, 100.0),
        (110.0, 90.0),
        false,
        false,
    );
    assert!(matches!(
        moved.geometry,
        RulerGeometry::Angle {
            points: [(110.0, 90.0), (200.0, 100.0), (170.0, 180.0)],
            ..
        }
    ));
}

/// 【尺子测试】【旋转锚点】旋转长度框拖动一角时，对角保持物理位置不变
#[test]
fn rotated_length_resize_keeps_the_opposite_corner_stationary() {
    let ruler = length(35.0);
    let original = ruler.corners().unwrap();
    let changed = drag::dragged(
        ruler,
        SelectionHandle::TopLeft,
        original[0],
        (original[0].0 - 30.0, original[0].1 - 20.0),
        false,
        false,
    );
    let corners = changed.corners().unwrap();
    assert!(distance(corners[2], original[2]) < 0.0001);
    assert!(distance(corners[0], (original[0].0 - 30.0, original[0].1 - 20.0)) < 0.0001);
    assert_ne!(changed.length_metrics(), ruler.length_metrics());
}

/// 【尺子测试】【等比角度框】Ctrl 同比调整三个定点，保持夹角且不改变样式粗细
#[test]
fn proportional_angle_resize_preserves_the_angle() {
    let ruler = angle();
    let frame = drag::frame(ruler).unwrap();
    let resized = drag::dragged(
        ruler,
        SelectionHandle::BottomRight,
        (frame.right(), frame.bottom()),
        (300.0 + ANGLE_FRAME_PADDING, 200.0 + ANGLE_FRAME_PADDING),
        true,
        false,
    );
    assert_eq!(ruler.info_text(), resized.info_text());
    assert_eq!(ruler.unit(3.0), resized.unit(3.0));
    let RulerGeometry::Angle { points, .. } = resized.geometry else {
        panic!("angle type changed")
    };
    assert_eq!(points[0], (100.0, 100.0));
    assert_eq!(points[1], (300.0, 100.0));
    assert_eq!(points[2], (240.0, 260.0));
}

/// 【尺子测试】【模式旋转】旋转把手只在选中后可命中，Shift 旋转吸附到 45 度方向
#[test]
fn rotation_handles_require_selection_and_can_snap() {
    let ruler = length(0.0);
    let handle = drag::rotation_handle(ruler, 3.0).unwrap();
    assert_eq!(
        drag::hit_test(ruler, handle, 3.0, false, true),
        SelectionHandle::None
    );
    assert_eq!(
        drag::hit_test(ruler, handle, 3.0, true, true),
        SelectionHandle::Rotate
    );
    let rotated = drag::dragged(
        ruler,
        SelectionHandle::Rotate,
        handle,
        (250.0, 100.0),
        false,
        true,
    );
    assert_eq!(rotated.length_metrics(), ruler.length_metrics());
    assert!(matches!(
        rotated.geometry,
        RulerGeometry::Length { rotation: 90.0, .. }
    ));
    let ruler = angle();
    let handle = drag::rotation_handle(ruler, 3.0).unwrap();
    let rotated = drag::dragged(
        ruler,
        SelectionHandle::Rotate,
        handle,
        (250.0, 100.0),
        false,
        true,
    );
    assert_eq!(rotated.info_text(), ruler.info_text());
}

/// 【尺子测试】【平直角度框】共线角度不在退化轴上扩张，保持可操作范围而不制造新夹角
#[test]
fn a_flat_angle_frame_keeps_its_flat_axis() {
    let mut ruler = angle();
    ruler.geometry = RulerGeometry::Angle {
        points: [(100.0, 100.0), (150.0, 100.0), (200.0, 100.0)],
        count: 3,
    };
    let frame = drag::frame(ruler).unwrap();
    let resized = drag::dragged(
        ruler,
        SelectionHandle::BottomRight,
        (frame.right(), frame.bottom()),
        (300.0, 200.0),
        true,
        false,
    );
    let RulerGeometry::Angle { points, .. } = resized.geometry else {
        panic!("angle type changed")
    };
    assert!(points.iter().all(|point| point.1 == 100.0));
    assert_eq!(resized.info_text().as_deref(), Some("0.0\u{b0}"));
}
