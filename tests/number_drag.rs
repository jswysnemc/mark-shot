//! 【序号测试】【定点交互】验证圆心、尖端、原始快照缩放和旋转控制

use mark_shot::features::number::{NumberMark, NumberStyle, drag};
use mark_shot::types::SelectionHandle;
use tiny_skia::Rect;

/// 【序号测试】【标记构造】返回带六十度旋转和独立尖端的字母序号
fn mark() -> NumberMark {
    let mut mark = NumberMark::new((140.0, 140.0), 7, NumberStyle::UpperAlpha).unwrap();
    mark.tip = (280.0, 140.0);
    mark.rotation = std::f32::consts::PI / 3.0;
    mark
}

/// 【序号测试】【绘图范围】返回四百像素见方的原图范围
fn bounds() -> Rect {
    Rect::from_xywh(0.0, 0.0, 400.0, 400.0).unwrap()
}

/// 【序号测试】【定点比较】验证 actual、expected 两点在万分之一像素内重合
fn near(actual: (f32, f32), expected: (f32, f32)) {
    assert!(
        (actual.0 - expected.0).abs() < 0.0001 && (actual.1 - expected.1).abs() < 0.0001,
        "{actual:?} != {expected:?}"
    );
}

/// 【序号测试】【实际命中】控制点位于旋转后的实际位置，未选中对象不暴露外框控制点
#[test]
fn handles_follow_visual_positions_and_selection_state() {
    let mark = mark();
    assert_eq!(
        drag::hit_test(mark, 3.0, mark.bubble, true),
        SelectionHandle::NumberBubble
    );
    assert_eq!(
        drag::hit_test(mark, 3.0, mark.to_visual(mark.tip), true),
        SelectionHandle::NumberTip
    );
    assert_eq!(
        drag::hit_test(mark, 3.0, mark.tip, false),
        SelectionHandle::None
    );
    assert_eq!(drag::controls(mark, 3.0).len(), 10);
    let handle = drag::rotation_handle(mark, 3.0).unwrap();
    assert_eq!(
        drag::hit_test(mark, 3.0, handle, true),
        SelectionHandle::Rotate
    );
    assert_eq!(
        drag::hit_test(mark, 3.0, handle, false),
        SelectionHandle::None
    );
}

/// 【序号测试】【圆心移动】拖动圆心不会移动原尖端，旋转角度及样式不变
#[test]
fn moving_the_bubble_preserves_the_visual_tip() {
    let before = mark();
    let (after, width) = drag::dragged(
        before,
        3.0,
        SelectionHandle::NumberBubble,
        before.bubble,
        (200.0, 170.0),
        bounds(),
        false,
        false,
    );
    assert_eq!(after.bubble, (200.0, 170.0));
    near(after.to_visual(after.tip), before.to_visual(before.tip));
    assert_eq!(after.rotation, before.rotation);
    assert_eq!(after.number, before.number);
    assert_eq!(width, 3.0);
}

/// 【序号测试】【尖端移动】拖动尖端精确跟随实际指针，圆心不移动，范围外终点约束到原图
#[test]
fn moving_the_tip_tracks_the_pointer_without_moving_the_bubble() {
    let before = mark();
    let (after, _) = drag::dragged(
        before,
        3.0,
        SelectionHandle::NumberTip,
        before.to_visual(before.tip),
        (370.0, 30.0),
        bounds(),
        false,
        false,
    );
    near(after.to_visual(after.tip), (370.0, 30.0));
    assert_eq!(after.bubble, before.bubble);
    let (after, _) = drag::dragged(
        before,
        3.0,
        SelectionHandle::NumberTip,
        before.to_visual(before.tip),
        (500.0, -20.0),
        bounds(),
        false,
        false,
    );
    near(after.to_visual(after.tip), (400.0, 0.0));
}

/// 【序号测试】【重合定点】单击创建的重合定点优先命中圆心，移动圆心后可以形成指向尾部
#[test]
fn click_bubbles_remain_editable_without_an_existing_leader() {
    let before = NumberMark::new((100.0, 100.0), 1, NumberStyle::Arabic).unwrap();
    assert_eq!(
        drag::hit_test(before, 3.0, before.bubble, true),
        SelectionHandle::NumberBubble
    );
    let (after, _) = drag::dragged(
        before,
        3.0,
        SelectionHandle::NumberBubble,
        before.bubble,
        (190.0, 100.0),
        bounds(),
        false,
        false,
    );
    assert_eq!(after.tip, before.tip);
    assert!(after.tail(3.0).is_some());
}

/// 【序号测试】【旋转吸附】旋转保持圆心，Shift 将角度吸附到四十五度倍数
#[test]
fn rotation_is_bubble_centered_and_can_snap_to_forty_five_degrees() {
    let mut before = mark();
    before.rotation = 0.0;
    let (after, _) = drag::dragged(
        before,
        3.0,
        SelectionHandle::Rotate,
        (240.0, 140.0),
        (225.0, 205.0),
        bounds(),
        false,
        true,
    );
    near(after.bubble, before.bubble);
    assert!((after.rotation - std::f32::consts::FRAC_PI_4).abs() < 0.0001);
}

/// 【序号测试】【整体平移】从选中外框移动时两点整体平移，靠近原图边界时不丢失旋转关系
#[test]
fn whole_mark_moves_keep_both_visual_points_in_the_source() {
    let before = mark();
    let (after, _) = drag::dragged(
        before,
        3.0,
        SelectionHandle::Move,
        (180.0, 180.0),
        (800.0, 800.0),
        bounds(),
        false,
        false,
    );
    let tip_before = before.to_visual(before.tip);
    let tip_after = after.to_visual(after.tip);
    near(
        (tip_after.0 - after.bubble.0, tip_after.1 - after.bubble.1),
        (
            tip_before.0 - before.bubble.0,
            tip_before.1 - before.bubble.1,
        ),
    );
    assert!(tip_after.0 <= 400.0001 && tip_after.1 <= 400.0001);
    assert!(after.bubble.0 <= 400.0 && after.bubble.1 <= 400.0);
}

/// 【序号测试】【外框缩放】外框控制点可操作，缩放增加独立笔宽但不改变编号、格式或导出比例
#[test]
fn frame_resize_uses_the_original_snapshot_and_preserves_semantics() {
    let mut before = mark();
    before.rotation = 0.0;
    let handle = drag::controls(before, 3.0)
        .into_iter()
        .find(|(handle, _)| *handle == SelectionHandle::BottomRight)
        .unwrap()
        .1;
    assert_eq!(
        drag::hit_test(before, 3.0, handle, true),
        SelectionHandle::BottomRight
    );
    let (after, width) = drag::dragged(
        before,
        3.0,
        SelectionHandle::BottomRight,
        handle,
        (handle.0 + 60.0, handle.1 + 40.0),
        bounds(),
        true,
        false,
    );
    assert!(width > 3.0 && width <= 72.0);
    assert_eq!(after.number, before.number);
    assert_eq!(after.style, before.style);
    assert_eq!(after.style_scale, before.style_scale);
    let repeated = drag::dragged(
        before,
        3.0,
        SelectionHandle::BottomRight,
        handle,
        (handle.0 + 60.0, handle.1 + 40.0),
        bounds(),
        true,
        false,
    );
    assert_eq!(repeated, (after, width));
}
