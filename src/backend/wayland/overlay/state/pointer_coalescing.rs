//! 【Wayland覆盖层】【移动合并】保留最终坐标及有界真实轨迹，不增加逐点重绘

use crate::types::{OverlayEvent, pointer_motion::PointerMotion};
use std::collections::VecDeque;

const MAX_HISTORY: usize = 128;

/// 【Wayland覆盖层】【轨迹合并】把range按实际输入顺序纳入history，仅移除相邻重复采样，无返回值
fn include(history: &mut Vec<PointerMotion>, range: PointerMotion) {
    if history.last() != Some(&range) {
        history.push(range);
    }
}

/// 【Wayland覆盖层】【连续移动】合并 first 后排队的移动与帧回调，返回最后坐标和完整范围，按钮及焦点事件保持顺序
pub(in crate::backend::wayland::overlay) fn coalesce(
    first: OverlayEvent,
    events: &mut VecDeque<OverlayEvent>,
) -> OverlayEvent {
    let OverlayEvent::PointerMove {
        monitor_idx,
        x,
        y,
        mut history,
    } = first
    else {
        return first;
    };
    let mut latest = (monitor_idx, x, y);
    // 1. 【Wayland覆盖层】【范围保留】只在发生合并时分配范围列表，普通单次移动不增加堆分配
    while let Some(OverlayEvent::PointerMove { .. } | OverlayEvent::FrameDone { .. }) =
        events.front()
    {
        // 2. 【Wayland覆盖层】【有界处理】高密度非共线输入分批分派，按钮及未消费坐标继续保留队列顺序
        if let Some(OverlayEvent::PointerMove {
            history: previous, ..
        }) = events.front()
            && history.len() + previous.len() + 2 > MAX_HISTORY
        {
            break;
        }
        if let OverlayEvent::PointerMove {
            monitor_idx,
            x,
            y,
            history: previous,
        } = events.pop_front().unwrap()
        {
            include(
                &mut history,
                PointerMotion::new(latest.0, (latest.1, latest.2)),
            );
            for range in previous {
                include(&mut history, range);
            }
            include(&mut history, PointerMotion::new(monitor_idx, (x, y)));
            latest = (monitor_idx, x, y);
        }
    }
    OverlayEvent::PointerMove {
        monitor_idx: latest.0,
        x: latest.1,
        y: latest.2,
        history,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【移动合并测试】【输入构造】按输出和坐标返回真实事件，未合并时范围为空
    fn motion(monitor_idx: usize, x: f64, y: f64) -> OverlayEvent {
        OverlayEvent::PointerMove {
            monitor_idx,
            x,
            y,
            history: Vec::new(),
        }
    }

    /// 【移动合并测试】【移出返回】保留远离后返回的极值及按钮顺序，最终指针仍位于原坐标
    #[test]
    fn returning_pointer_keeps_excursion_and_button_boundary() {
        let mut events = VecDeque::from([
            motion(0, 24.0, 10.0),
            OverlayEvent::FrameDone { monitor_idx: 0 },
            motion(0, 39.0, 10.0),
            OverlayEvent::PointerButton {
                button: crate::types::MouseButton::Left,
                pressed: true,
                serial: None,
                timestamp: Some(42),
            },
            motion(0, 80.0, 10.0),
        ]);
        let OverlayEvent::PointerMove { x, y, history, .. } =
            coalesce(motion(0, 39.0, 10.0), &mut events)
        else {
            panic!("expected pointer move")
        };
        assert_eq!((x, y), (39.0, 10.0));
        assert_eq!(
            history,
            [
                PointerMotion::new(0, (39.0, 10.0)),
                PointerMotion::new(0, (24.0, 10.0)),
                PointerMotion::new(0, (39.0, 10.0))
            ]
        );
        assert!(matches!(
            events.front(),
            Some(OverlayEvent::PointerButton { .. })
        ));
        assert_eq!(events.len(), 2);
    }

    /// 【移动合并测试】【多输出范围】不同输出使用独立局部范围，二次合并保留已压缩轨迹
    #[test]
    fn separate_output_bounds_survive_repeated_coalescing() {
        let first = coalesce(
            motion(1, 9.0, -2.0),
            &mut VecDeque::from([motion(1, 20.0, 5.0)]),
        );
        let mut events = VecDeque::from([motion(0, 2.0, 7.0), motion(1, 10.0, 0.0)]);
        let OverlayEvent::PointerMove {
            monitor_idx,
            history,
            ..
        } = coalesce(first, &mut events)
        else {
            panic!("expected pointer move")
        };
        assert_eq!(monitor_idx, 1);
        assert_eq!(
            history,
            [
                PointerMotion::new(1, (9.0, -2.0)),
                PointerMotion::new(1, (20.0, 5.0)),
                PointerMotion::new(0, (2.0, 7.0)),
                PointerMotion::new(1, (10.0, 0.0)),
            ]
        );
        let OverlayEvent::PointerMove { history, .. } =
            coalesce(motion(0, 1.0, 2.0), &mut VecDeque::new())
        else {
            panic!("expected pointer move")
        };
        assert!(history.is_empty());
    }

    /// 【移动合并测试】【真实角点】非共线轨迹不能拼出未经过的对角线位置，无参数、无返回值
    #[test]
    fn diagonal_motion_keeps_actual_points() {
        let mut events = VecDeque::from([motion(0, 130.0, 124.0), motion(0, 131.0, 121.0)]);
        let OverlayEvent::PointerMove { history, .. } =
            coalesce(motion(0, 134.0, 120.0), &mut events)
        else {
            panic!("expected pointer move")
        };
        assert!(
            history
                .iter()
                .all(|range| { range.min.0 == range.max.0 || range.min.1 == range.max.1 })
        );
        for range in history {
            for point in [range.min, range.max] {
                assert!((point.0 - 130.0).hypot(point.1 - 120.0) < 5.0);
            }
        }
    }

    /// 【移动合并测试】【有界轨迹】高密度非共线输入分批处理，保留未消费事件且不无限分配，无参数、无返回值
    #[test]
    fn dense_motion_keeps_remaining_events_in_order() {
        let mut events = (1..500)
            .map(|index| motion(0, f64::from(index), f64::from(index)))
            .collect();
        let OverlayEvent::PointerMove { history, x, .. } =
            coalesce(motion(0, 0.0, 0.0), &mut events)
        else {
            panic!("expected pointer move")
        };
        assert!(history.len() <= 128);
        assert!(!events.is_empty());
        let Some(OverlayEvent::PointerMove { x: next, .. }) = events.front() else {
            panic!("expected remaining pointer move")
        };
        assert_eq!(*next, x + 1.0);
    }
}
