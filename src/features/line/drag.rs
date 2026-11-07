//! 【线条】【定点拖动】在原始骨架上调整端点、控制点、外框和旋转，避免累计误差

use tiny_skia::Rect;

use super::{LineSkeleton, Point, geometry, polyline::distance};
use crate::types::{SelectionHandle, SignedRect};
use crate::utils::{apply_handle_drag, hit_test_rect_handle};

const FRAME_PADDING: f32 = 0.0;
const POINT_HIT: f32 = 10.0;

/// 【线条】【外框几何】按bbox返回生产原图选框，不额外扩容，返回范围或空值
pub fn frame(bbox: Rect) -> Option<Rect> {
    bbox.outset(FRAME_PADDING, FRAME_PADDING)
}

/// 【线条】【旋转位置】以 skeleton 全部定点和 bbox 计算上方旋转坐标；返回不与外部控制点重叠的把手位置
pub fn rotation_point(skeleton: &LineSkeleton, bbox: Rect) -> Point {
    let top = skeleton
        .points()
        .map(|point| point.1)
        .fold(bbox.top(), f32::min);
    (
        (bbox.left() + bbox.right()) * 0.5,
        top - FRAME_PADDING - 26.0,
    )
}

/// 【线条】【编辑损坏范围】为 skeleton、bbox 合并外框、旋转把手和定点，frame_pad 计入统一描边与阴影；返回完整编辑边界
pub fn editing_bounds(skeleton: &LineSkeleton, bbox: Rect, frame_pad: f32) -> Option<Rect> {
    if !frame_pad.is_finite() {
        return None;
    }
    let frame = frame(bbox)?.outset(frame_pad.max(0.0), frame_pad.max(0.0))?;
    let rotation = rotation_point(skeleton, bbox);
    let mut bounds = (
        frame.left(),
        frame.top().min(rotation.1 - 9.0),
        frame.right(),
        frame.bottom(),
    );
    for point in skeleton.points() {
        bounds.0 = bounds.0.min(point.0 - 9.0);
        bounds.1 = bounds.1.min(point.1 - 9.0);
        bounds.2 = bounds.2.max(point.0 + 9.0);
        bounds.3 = bounds.3.max(point.1 + 9.0);
    }
    Rect::from_ltrb(
        bounds.0 - 2.0,
        bounds.1 - 2.0,
        bounds.2 + 2.0,
        bounds.3 + 2.0,
    )
}

/// 【线条】【控制点命中】在 skeleton、bbox 下判断 point；selected 决定控制点与外框是否可见，width 提供路径容差
pub fn hit_test(
    skeleton: &LineSkeleton,
    bbox: Rect,
    width: f32,
    point: Point,
    selected: bool,
) -> SelectionHandle {
    if !skeleton.valid() || !point.0.is_finite() || !point.1.is_finite() {
        return SelectionHandle::None;
    }
    let candidate = skeleton
        .points()
        .enumerate()
        .filter(|(index, _)| selected || *index < 2)
        .map(|(index, anchor)| (index, distance(anchor, point)))
        .filter(|(_, distance)| *distance <= POINT_HIT)
        .min_by(|first, second| first.1.total_cmp(&second.1));
    if let Some((index, _)) = candidate {
        return SelectionHandle::LinePoint(index);
    }
    if selected {
        if distance(rotation_point(skeleton, bbox), point) <= 9.0 {
            return SelectionHandle::Rotate;
        }
        if let Some(frame) = frame(bbox) {
            let handle = hit_test_rect_handle(&frame, (point.0 as f64, point.1 as f64));
            if handle != SelectionHandle::None {
                return handle;
            }
        }
    }
    if geometry::insertion_at(skeleton, point, POINT_HIT + width * 0.5).is_some() {
        SelectionHandle::Move
    } else {
        SelectionHandle::None
    }
}

/// 【线条】【原始快照拖动】按 handle、start、point 在 bounds 中变换 before；ctrl 保持缩放比例，shift 吸附旋转，返回新骨架
pub fn dragged(
    before: &LineSkeleton,
    bbox: Rect,
    handle: SelectionHandle,
    start: Point,
    point: Point,
    bounds: Rect,
    ctrl: bool,
    shift: bool,
) -> LineSkeleton {
    if !before.valid() || !point.0.is_finite() || !point.1.is_finite() {
        return before.clone();
    }
    let mut result = before.clone();
    match handle {
        SelectionHandle::LinePoint(index) => {
            result.set_point(
                index,
                (
                    point.0.clamp(bounds.left(), bounds.right()),
                    point.1.clamp(bounds.top(), bounds.bottom()),
                ),
            );
        }
        SelectionHandle::Move => {
            let min_x = before
                .points()
                .map(|point| point.0)
                .fold(f32::INFINITY, f32::min);
            let max_x = before
                .points()
                .map(|point| point.0)
                .fold(f32::NEG_INFINITY, f32::max);
            let min_y = before
                .points()
                .map(|point| point.1)
                .fold(f32::INFINITY, f32::min);
            let max_y = before
                .points()
                .map(|point| point.1)
                .fold(f32::NEG_INFINITY, f32::max);
            let x_range = (bounds.left() - min_x, bounds.right() - max_x);
            let y_range = (bounds.top() - min_y, bounds.bottom() - max_y);
            let dx = (point.0 - start.0).clamp(x_range.0.min(x_range.1), x_range.0.max(x_range.1));
            let dy = (point.1 - start.1).clamp(y_range.0.min(y_range.1), y_range.0.max(y_range.1));
            result = before.mapped(|point| (point.0 + dx, point.1 + dy));
        }
        SelectionHandle::Rotate => {
            let center = (
                (bbox.left() + bbox.right()) * 0.5,
                (bbox.top() + bbox.bottom()) * 0.5,
            );
            let first_angle = (start.1 - center.1).atan2(start.0 - center.0);
            let angle = (point.1 - center.1).atan2(point.0 - center.0) - first_angle;
            let bearing = (before.end.1 - before.start.1).atan2(before.end.0 - before.start.0);
            let angle = if shift {
                ((bearing + angle) / std::f32::consts::FRAC_PI_4).round()
                    * std::f32::consts::FRAC_PI_4
                    - bearing
            } else {
                angle
            };
            let (sin, cos) = angle.sin_cos();
            result = before.mapped(|point| {
                let delta = (point.0 - center.0, point.1 - center.1);
                (
                    center.0 + delta.0 * cos - delta.1 * sin,
                    center.1 + delta.0 * sin + delta.1 * cos,
                )
            });
        }
        SelectionHandle::TopLeft
        | SelectionHandle::Top
        | SelectionHandle::TopRight
        | SelectionHandle::Left
        | SelectionHandle::Right
        | SelectionHandle::BottomLeft
        | SelectionHandle::Bottom
        | SelectionHandle::BottomRight => {
            if let Some(frame) = frame(bbox) {
                let resized = apply_handle_drag(
                    &frame,
                    handle,
                    ((point.0 - start.0) as f64, (point.1 - start.1) as f64),
                );
                result = resize(before, frame, resized, handle, ctrl);
            }
        }
        _ => {}
    }
    if result.valid() {
        result
    } else {
        before.clone()
    }
}

/// 【线条】【外框缩放】把 before 的全部定点从 frame 映射到 resized；handle 和 ctrl 约束比例，返回新骨架，笔宽不变
fn resize(
    before: &LineSkeleton,
    frame: Rect,
    mut resized: SignedRect,
    handle: SelectionHandle,
    ctrl: bool,
) -> LineSkeleton {
    let mut sx = resized.width() / frame.width();
    let mut sy = resized.height() / frame.height();
    if ctrl {
        let scale = match handle {
            SelectionHandle::Left | SelectionHandle::Right => sx.abs(),
            SelectionHandle::Top | SelectionHandle::Bottom => sy.abs(),
            _ => sx.abs().max(sy.abs()),
        }
        .max(0.01);
        sx = scale.copysign(sx);
        sy = scale.copysign(sy);
        if matches!(
            handle,
            SelectionHandle::TopLeft | SelectionHandle::Left | SelectionHandle::BottomLeft
        ) {
            resized.left = resized.right - frame.width() * sx;
        }
        if matches!(
            handle,
            SelectionHandle::TopLeft | SelectionHandle::Top | SelectionHandle::TopRight
        ) {
            resized.top = resized.bottom - frame.height() * sy;
        }
    }
    // 1. 【线条】【外框缩放】始终从快照映射端点和控制点，反向缩放不依赖上一帧结果
    before.mapped(|point| {
        (
            resized.left + (point.0 - frame.left()) * sx,
            resized.top + (point.1 - frame.top()) * sy,
        )
    })
}
