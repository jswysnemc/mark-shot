//! 【工具提示】【组件回归】验证稳定悬停期限、关闭和显示器边缘避让

use super::{DELAY, State, Target, View, place};
use crate::ui::toolbar::ToolbarButton;
use std::time::{Duration, Instant};
use tiny_skia::Rect;

/// 【工具提示】【目标构造】按 x、y 和 vertical 返回固定按钮及所在工具栏的测试目标
fn target(x: f32, y: f32, vertical: bool) -> Target {
    Target {
        button: ToolbarButton::Undo,
        secondary: false,
        monitor_idx: 0,
        cell: Rect::from_xywh(x, y, 30.0, 30.0).unwrap(),
        toolbar: Rect::from_xywh(x, y, 42.0, 42.0).unwrap(),
        vertical,
    }
}

/// 【工具提示】【稳定期限】无参数，同一目标更新不延后期限，显示后没有周期唤醒，关闭时返回旧损坏区域
#[test]
fn stationary_hover_has_one_deadline_and_close_restores_the_old_area() {
    let mut state = State::default();
    let now = Instant::now();
    let target = target(100.0, 100.0, false);
    state.hover(Some(target), now);
    state.hover(Some(target), now + Duration::from_millis(300));
    assert_eq!(state.deadline(), Some(now + DELAY));
    assert!(!state.due(now + DELAY - Duration::from_millis(1)));
    assert!(state.due(now + DELAY));
    let rect = place(target, 80.0, 14.0, (640.0, 480.0)).unwrap();
    state.show(View {
        text: "Undo (Ctrl+Z)".into(),
        rect,
        monitor_idx: 0,
        cache: None,
        theme: None,
    });
    assert_eq!(state.deadline(), None);
    assert_eq!(state.close(), Some((0, rect)));
    assert_eq!(state.target(), None);
    assert!(!state.due(now + DELAY * 2));
}

/// 【工具提示】【目标切换】无参数，切换按钮或布局后必须重新等待完整延迟
#[test]
fn changing_buttons_or_layout_restarts_the_delay() {
    let mut state = State::default();
    let now = Instant::now();
    state.hover(Some(target(100.0, 100.0, false)), now);
    let next = now + Duration::from_millis(300);
    state.hover(Some(target(130.0, 100.0, false)), next);
    assert!(!state.due(now + DELAY));
    assert_eq!(state.deadline(), Some(next + DELAY));
}

/// 【工具提示】【边缘避让】无参数，顶部、底部及左右竖栏提示均位于显示器内，优先选择不覆盖工具栏的一侧
#[test]
fn tooltips_flip_at_bottom_and_right_edges_and_fit_narrow_outputs() {
    for (target, size) in [
        (target(8.0, 8.0, false), (640.0, 480.0)),
        (target(590.0, 430.0, false), (640.0, 480.0)),
        (target(8.0, 200.0, true), (640.0, 480.0)),
        (target(590.0, 200.0, true), (640.0, 480.0)),
        (target(8.0, 8.0, false), (100.0, 120.0)),
    ] {
        let rect = place(target, 160.0, 14.0, size).unwrap();
        assert!(rect.left() >= 8.0 && rect.top() >= 8.0);
        assert!(rect.right() <= size.0 - 8.0 && rect.bottom() <= size.1 - 8.0);
        assert!(!crate::utils::rects_overlap(&rect, &target.toolbar));
    }
}
