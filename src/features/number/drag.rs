//! 【序号】【控制点拖动】独立调整圆心和尖端，围绕圆心旋转并按原始快照缩放

use tiny_skia::Rect;

use super::{NumberMark, clamped_width};
use crate::types::SelectionHandle;
use crate::utils::apply_handle_drag;

/// 【序号】【控制点列表】按 mark、width 返回八个外框控制点及圆心、实际尖端；无效几何返回空列表
pub fn controls(mark: NumberMark, width: f32) -> Vec<(SelectionHandle, (f32, f32))> {
    let Some(frame) = mark.unrotated_bounds(width) else {
        return Vec::new();
    };
    let (left, top, right, bottom) = (frame.left(), frame.top(), frame.right(), frame.bottom());
    let middle = ((left + right) / 2.0, (top + bottom) / 2.0);
    let mut controls = [
        (SelectionHandle::TopLeft, (left, top)),
        (SelectionHandle::Top, (middle.0, top)),
        (SelectionHandle::TopRight, (right, top)),
        (SelectionHandle::Left, (left, middle.1)),
        (SelectionHandle::Right, (right, middle.1)),
        (SelectionHandle::BottomLeft, (left, bottom)),
        (SelectionHandle::Bottom, (middle.0, bottom)),
        (SelectionHandle::BottomRight, (right, bottom)),
    ]
    .map(|(handle, point)| (handle, mark.to_visual(point)))
    .to_vec();
    controls.push((SelectionHandle::NumberBubble, mark.bubble));
    controls.push((SelectionHandle::NumberTip, mark.to_visual(mark.tip)));
    controls
}

/// 【序号】【旋转把手】按mark、width返回圆框上方二十六样式单位的旋转把手；无效几何返回空值
pub fn rotation_handle(mark: NumberMark, width: f32) -> Option<(f32, f32)> {
    let frame = mark.unrotated_bounds(width)?;
    Some(mark.to_visual((
        (frame.left() + frame.right()) / 2.0,
        frame.top() - 26.0 * mark.style_scale,
    )))
}

/// 【序号】【控制点命中】按 mark、width 判断实际 point 的操作，selected 控制外框及旋转把手；返回操作或空控制点
pub fn hit_test(
    mark: NumberMark,
    width: f32,
    point: (f32, f32),
    selected: bool,
) -> SelectionHandle {
    if !mark.valid() || !point.0.is_finite() || !point.1.is_finite() {
        return SelectionHandle::None;
    }
    let tolerance = 8.0 * mark.style_scale;
    if selected
        && rotation_handle(mark, width)
            .is_some_and(|handle| distance(handle, point) <= tolerance * 1.4)
    {
        return SelectionHandle::Rotate;
    }
    let tip = mark.to_visual(mark.tip);
    let radius = mark.bubble_radius(width);
    // 1. 【序号】【控制点命中】独立尖端优先于邻近外框；单击圆框的重合尖端仍交给圆心
    if distance(tip, mark.bubble) > radius + tolerance && distance(tip, point) <= tolerance * 1.6 {
        return SelectionHandle::NumberTip;
    }
    if selected {
        for (handle, position) in controls(mark, width).into_iter().take(8) {
            if distance(position, point) <= tolerance {
                return handle;
            }
        }
    }
    if distance(mark.bubble, point) <= radius + tolerance {
        return SelectionHandle::NumberBubble;
    }
    if distance(tip, point) <= tolerance * 1.6 {
        return SelectionHandle::NumberTip;
    }
    let local = mark.to_local(point);
    if selected
        && mark.unrotated_bounds(width).is_some_and(|frame| {
            local.0 >= frame.left()
                && local.0 <= frame.right()
                && local.1 >= frame.top()
                && local.1 <= frame.bottom()
        })
    {
        return SelectionHandle::Move;
    }
    SelectionHandle::None
}

/// 【序号】【几何拖动】以 before、width 快照按 handle 从 start 拖到 point；bounds 约束实际定点，ctrl 等比、shift 吸附旋转，返回新标记和笔宽
pub fn dragged(
    before: NumberMark,
    width: f32,
    handle: SelectionHandle,
    start: (f32, f32),
    point: (f32, f32),
    bounds: Rect,
    ctrl: bool,
    shift: bool,
) -> (NumberMark, f32) {
    let width = clamped_width(width, before.style_scale);
    if !before.valid()
        || ![start.0, start.1, point.0, point.1]
            .into_iter()
            .all(f32::is_finite)
    {
        return (before, width);
    }
    let point = clamp_point(point, bounds);
    match handle {
        SelectionHandle::NumberTip => (
            NumberMark {
                tip: before.to_local(point),
                ..before
            },
            width,
        ),
        SelectionHandle::NumberBubble => {
            let visual_tip = before.to_visual(before.tip);
            let mark = NumberMark {
                bubble: point,
                ..before
            };
            // 1. 【序号】【圆心拖动】圆心改变后反解原尖端，保持尖端实际位置和旋转角度
            (
                NumberMark {
                    tip: mark.to_local(visual_tip),
                    ..mark
                },
                width,
            )
        }
        SelectionHandle::Rotate => {
            let a = (start.1 - before.bubble.1).atan2(start.0 - before.bubble.0);
            let b = (point.1 - before.bubble.1).atan2(point.0 - before.bubble.0);
            let mut rotation = before.rotation + b - a;
            if shift {
                let interval = std::f32::consts::FRAC_PI_4;
                rotation = (rotation / interval).round() * interval;
            }
            (
                constrained(NumberMark { rotation, ..before }, bounds),
                width,
            )
        }
        SelectionHandle::Move => {
            let tip = before.to_visual(before.tip);
            let limits = (
                bounds.left() - before.bubble.0.min(tip.0),
                bounds.right() - before.bubble.0.max(tip.0),
                bounds.top() - before.bubble.1.min(tip.1),
                bounds.bottom() - before.bubble.1.max(tip.1),
            );
            if limits.0 > limits.1 || limits.2 > limits.3 {
                return (constrained(before, bounds), width);
            }
            let delta = (
                (point.0 - start.0).clamp(limits.0, limits.1),
                (point.1 - start.1).clamp(limits.2, limits.3),
            );
            (before.translated(delta), width)
        }
        SelectionHandle::None => (before, width),
        _ => resized(before, width, handle, start, point, bounds, ctrl),
    }
}

/// 【序号】【边框缩放】以 before 的未旋转边界响应 handle、start、point，ctrl 保持比例；bounds 约束定点，返回新标记和受限笔宽
fn resized(
    before: NumberMark,
    width: f32,
    handle: SelectionHandle,
    start: (f32, f32),
    point: (f32, f32),
    bounds: Rect,
    ctrl: bool,
) -> (NumberMark, f32) {
    let Some(frame) = before.unrotated_bounds(width) else {
        return (before, width);
    };
    let start = before.to_local(start);
    let point = before.to_local(point);
    let mut target = apply_handle_drag(
        &frame,
        handle,
        ((point.0 - start.0) as f64, (point.1 - start.1) as f64),
    );
    let sx = target.width().abs() / frame.width();
    let sy = target.height().abs() / frame.height();
    let factor = if ctrl {
        match handle {
            SelectionHandle::Left | SelectionHandle::Right => sx,
            SelectionHandle::Top | SelectionHandle::Bottom => sy,
            _ => sx.max(sy),
        }
    } else {
        sx.max(sy)
    }
    .max(0.01);
    if ctrl {
        let width = frame.width() * factor;
        let height = frame.height() * factor;
        if matches!(
            handle,
            SelectionHandle::TopLeft | SelectionHandle::Left | SelectionHandle::BottomLeft
        ) {
            target.left = target.right - width;
        } else {
            target.right = target.left + width;
        }
        if matches!(
            handle,
            SelectionHandle::TopLeft | SelectionHandle::Top | SelectionHandle::TopRight
        ) {
            target.top = target.bottom - height;
        } else {
            target.bottom = target.top + height;
        }
    }
    let sx = target.width() / frame.width();
    let sy = target.height() / frame.height();
    let map = |point: (f32, f32)| {
        (
            target.left + (point.0 - frame.left()) * sx,
            target.top + (point.1 - frame.top()) * sy,
        )
    };
    let mark = NumberMark {
        bubble: map(before.bubble),
        tip: map(before.tip),
        ..before
    };
    (
        constrained(mark, bounds),
        clamped_width(width * factor, before.style_scale),
    )
}

/// 【序号】【定点约束】将 mark 的圆心和实际尖端限制在 bounds 内，保留旋转关系；返回受限标记
fn constrained(mark: NumberMark, bounds: Rect) -> NumberMark {
    let tip = clamp_point(mark.to_visual(mark.tip), bounds);
    let mark = NumberMark {
        bubble: clamp_point(mark.bubble, bounds),
        ..mark
    };
    NumberMark {
        tip: mark.to_local(tip),
        ..mark
    }
}

/// 【序号】【坐标限制】将有限 point 限制在 bounds 内，返回实际逻辑坐标
fn clamp_point(point: (f32, f32), bounds: Rect) -> (f32, f32) {
    (
        point.0.clamp(bounds.left(), bounds.right()),
        point.1.clamp(bounds.top(), bounds.bottom()),
    )
}

/// 【序号】【定点距离】返回 a、b 两点间的欧氏距离
fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    (a.0 - b.0).hypot(a.1 - b.1)
}
