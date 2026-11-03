//! 【滤镜】【旋转编辑】独立处理旋转矩形的八个缩放点、移动和历史快照几何

use tiny_skia::Rect;

use super::{RegionFilter, model::Point};
use crate::types::{SelectionHandle, SignedRect};
use crate::utils::apply_handle_drag;

/// 【滤镜】【编辑外框】返回 filter 正文外六个原图像素的未旋转控制框；无效区域返回空值
pub fn frame(filter: RegionFilter) -> Option<Rect> {
    filter
        .rect()?
        .outset(6.0 * filter.style_scale, 6.0 * filter.style_scale)
}

/// 【滤镜】【缩放控制点】返回 filter 的八个实际视觉控制点；空区域返回空列表
pub fn controls(filter: RegionFilter) -> Vec<(SelectionHandle, Point)> {
    let Some(frame) = frame(filter) else {
        return Vec::new();
    };
    let center = (
        (frame.left() + frame.right()) / 2.0,
        (frame.top() + frame.bottom()) / 2.0,
    );
    [
        (SelectionHandle::TopLeft, (frame.left(), frame.top())),
        (SelectionHandle::Top, (center.0, frame.top())),
        (SelectionHandle::TopRight, (frame.right(), frame.top())),
        (SelectionHandle::Left, (frame.left(), center.1)),
        (SelectionHandle::Right, (frame.right(), center.1)),
        (SelectionHandle::BottomLeft, (frame.left(), frame.bottom())),
        (SelectionHandle::Bottom, (center.0, frame.bottom())),
        (
            SelectionHandle::BottomRight,
            (frame.right(), frame.bottom()),
        ),
    ]
    .map(|(handle, point)| (handle, filter.to_visual(point)))
    .to_vec()
}

/// 【滤镜】【旋转把手】返回 filter 上边中点外二十八个原图像素的视觉位置；空区域返回空值
pub fn rotation_handle(filter: RegionFilter) -> Option<Point> {
    let rect = filter.rect()?;
    Some(filter.to_visual((
        (rect.left() + rect.right()) / 2.0,
        rect.top() - 28.0 * filter.style_scale,
    )))
}

/// 【滤镜】【控制点命中】依据 filter、point、selected 返回操作；未显示的控制点不接受初次命中
pub fn hit_test(filter: RegionFilter, point: Point, selected: bool) -> SelectionHandle {
    if !filter.valid() || !point.0.is_finite() || !point.1.is_finite() {
        return SelectionHandle::None;
    }
    let tolerance = 8.0 * filter.style_scale;
    if selected {
        if rotation_handle(filter).is_some_and(|handle| distance(handle, point) <= tolerance * 1.4)
        {
            return SelectionHandle::Rotate;
        }
        for (handle, at) in controls(filter) {
            if distance(at, point) <= tolerance {
                return handle;
            }
        }
    }
    let local = filter.to_local(point);
    if filter.rect().is_some_and(|rect| {
        local.0 >= rect.left()
            && local.0 <= rect.right()
            && local.1 >= rect.top()
            && local.1 <= rect.bottom()
    }) {
        SelectionHandle::Move
    } else {
        SelectionHandle::None
    }
}

/// 【滤镜】【编辑损坏范围】返回 filter 正文、外框和旋转把手的总范围；空区域返回空值
pub fn editing_bounds(filter: RegionFilter) -> Option<Rect> {
    let mut bounds = filter
        .bounds()?
        .outset(10.0 * filter.style_scale, 10.0 * filter.style_scale)?;
    if let Some(handle) = rotation_handle(filter)
        && let Some(rect) = Rect::from_xywh(
            handle.0 - 12.0 * filter.style_scale,
            handle.1 - 12.0 * filter.style_scale,
            24.0 * filter.style_scale,
            24.0 * filter.style_scale,
        )
    {
        bounds = bounds.join(&rect)?;
    }
    Some(bounds)
}

/// 【滤镜】【拖动计算】按 before 快照、handle、start、point 修改区域；bounds 限制原图，ctrl 锁比例，shift 吸附旋转
pub fn dragged(
    before: RegionFilter,
    handle: SelectionHandle,
    start: Point,
    point: Point,
    bounds: Rect,
    ctrl: bool,
    shift: bool,
) -> RegionFilter {
    if !before.valid()
        || ![start.0, start.1, point.0, point.1]
            .into_iter()
            .all(f32::is_finite)
    {
        return before;
    }
    let Some(rect) = before.rect() else {
        return before;
    };
    match handle {
        SelectionHandle::Rotate => {
            let center = before.center();
            let first = (start.0 - center.0, start.1 - center.1);
            let second = (point.0 - center.0, point.1 - center.1);
            if first.0.hypot(first.1) < f32::EPSILON || second.0.hypot(second.1) < f32::EPSILON {
                return before;
            }
            let delta = (first.0 * second.1 - first.1 * second.0)
                .atan2(first.0 * second.0 + first.1 * second.1);
            let mut rotation = before.rotation + delta;
            if shift {
                rotation =
                    (rotation / std::f32::consts::FRAC_PI_4).round() * std::f32::consts::FRAC_PI_4;
            }
            RegionFilter {
                rotation: normalized_rotation(rotation),
                ..before
            }
        }
        SelectionHandle::Move => {
            let mut delta = (point.0 - start.0, point.1 - start.1);
            let visual = before.bounds().unwrap_or(rect);
            delta.0 = limited_delta(
                delta.0,
                rect.left(),
                rect.right(),
                visual.left(),
                visual.right(),
                bounds.left(),
                bounds.right(),
            );
            delta.1 = limited_delta(
                delta.1,
                rect.top(),
                rect.bottom(),
                visual.top(),
                visual.bottom(),
                bounds.top(),
                bounds.bottom(),
            );
            before.translated(delta)
        }
        SelectionHandle::TopLeft
        | SelectionHandle::Top
        | SelectionHandle::TopRight
        | SelectionHandle::Left
        | SelectionHandle::Right
        | SelectionHandle::BottomLeft
        | SelectionHandle::Bottom
        | SelectionHandle::BottomRight => {
            let start = before.to_local(start);
            let point = before.to_local(point);
            let mut target = apply_handle_drag(
                &rect,
                handle,
                (f64::from(point.0 - start.0), f64::from(point.1 - start.1)),
            );
            constrain_dimensions(
                &mut target,
                rect,
                handle,
                ctrl,
                2.0 * before.style_scale,
                bounds,
            );
            let local_center = (
                (target.left + target.right) / 2.0,
                (target.top + target.bottom) / 2.0,
            );
            // 1. 【滤镜】【拖动计算】新轴心映射到旧旋转空间，保持对侧视觉锚点不漂移
            let center = before.to_visual(local_center);
            let resized = RegionFilter {
                start: (
                    center.0 - target.width().abs() / 2.0,
                    center.1 - target.height().abs() / 2.0,
                ),
                end: (
                    center.0 + target.width().abs() / 2.0,
                    center.1 + target.height().abs() / 2.0,
                ),
                ..before
            };
            confine_source(resized, bounds)
        }
        _ => before,
    }
}

/// 【滤镜】【尺寸约束】按 original、handle、ctrl、minimum、bounds 调整 target；无返回值，强度不参与尺寸计算
fn constrain_dimensions(
    target: &mut SignedRect,
    original: Rect,
    handle: SelectionHandle,
    ctrl: bool,
    minimum: f32,
    bounds: Rect,
) {
    let mut width = target
        .width()
        .abs()
        .clamp(minimum.min(bounds.width()), bounds.width());
    let mut height = target
        .height()
        .abs()
        .clamp(minimum.min(bounds.height()), bounds.height());
    if ctrl {
        let requested = match handle {
            SelectionHandle::Left | SelectionHandle::Right => width / original.width(),
            SelectionHandle::Top | SelectionHandle::Bottom => height / original.height(),
            _ => (width / original.width()).max(height / original.height()),
        };
        let minimum_factor = (minimum / original.width()).max(minimum / original.height());
        let maximum_factor =
            (bounds.width() / original.width()).min(bounds.height() / original.height());
        let factor = requested.max(minimum_factor).min(maximum_factor);
        width = original.width() * factor;
        height = original.height() * factor;
    }
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

/// 【滤镜】【原图范围约束】平移 filter 使未旋转采样框位于 bounds 内；返回受限区域，不改变尺寸
fn confine_source(filter: RegionFilter, bounds: Rect) -> RegionFilter {
    let Some(rect) = filter.rect() else {
        return filter;
    };
    let delta = (
        (bounds.left() - rect.left()).max(0.0) - (rect.right() - bounds.right()).max(0.0),
        (bounds.top() - rect.top()).max(0.0) - (rect.bottom() - bounds.bottom()).max(0.0),
    );
    filter.translated(delta)
}

/// 【滤镜】【平移约束】把 delta 限制到原图与可用视觉边界；返回增量，旋转框无法完整容纳时优先保留源框
fn limited_delta(
    delta: f32,
    left: f32,
    right: f32,
    visual_left: f32,
    visual_right: f32,
    minimum: f32,
    maximum: f32,
) -> f32 {
    let low = (minimum - left).max(minimum - visual_left);
    let high = (maximum - right).min(maximum - visual_right);
    if low <= high {
        delta.clamp(low, high)
    } else {
        let (low, high) = (minimum - left, maximum - right);
        if low <= high {
            delta.clamp(low, high)
        } else {
            (minimum + maximum - left - right) / 2.0
        }
    }
}

/// 【滤镜】【旋转归一化】把 radians 限制到负半周至正半周；返回等价角度
fn normalized_rotation(radians: f32) -> f32 {
    (radians + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

/// 【滤镜】【控制点距离】返回 a 与 b 的欧氏距离，供控制点命中使用
fn distance(a: Point, b: Point) -> f32 {
    (a.0 - b.0).hypot(a.1 - b.1)
}
