//! 【尺子】【控制点变换】处理测量范围、定点、旋转把手与等比例缩放

use tiny_skia::Rect;

use super::angle_draft::constrained_point;
use super::geometry::{contains, distance, points_bounds, rect_center, rotated};
use super::{Measurement, Point, RulerGeometry, RulerMode};
use crate::types::SelectionHandle;

pub const HANDLE_TOLERANCE: f32 = 8.0;
pub const ANGLE_FRAME_PADDING: f32 = 14.0;

/// 【尺子】【选中范围】返回测量操作框；角度尺额外留出 padding，使范围控制点与三个定点分开
pub fn frame(measurement: Measurement) -> Option<Rect> {
    let bounds = measurement.unrotated_bounds()?;
    let padding = if measurement.mode() == RulerMode::Angle {
        ANGLE_FRAME_PADDING * measurement.style_scale
    } else {
        0.0
    };
    bounds.outset(padding, padding)
}

/// 【尺子】【范围控制点】返回 rect 四角与四边中点，按对应调整方式排列
pub fn frame_handles(rect: Rect) -> [(Point, SelectionHandle); 8] {
    let (x, y) = rect_center(rect);
    [
        ((rect.left(), rect.top()), SelectionHandle::TopLeft),
        ((rect.right(), rect.top()), SelectionHandle::TopRight),
        ((rect.left(), rect.bottom()), SelectionHandle::BottomLeft),
        ((rect.right(), rect.bottom()), SelectionHandle::BottomRight),
        ((x, rect.top()), SelectionHandle::Top),
        ((x, rect.bottom()), SelectionHandle::Bottom),
        ((rect.left(), y), SelectionHandle::Left),
        ((rect.right(), y), SelectionHandle::Right),
    ]
}

/// 【尺子】【定点控制点】根据 measurement 与 tolerance 返回可区分的长度控制点或角度定点
pub fn handles(measurement: Measurement, tolerance: f32) -> Vec<(Point, SelectionHandle)> {
    if let RulerGeometry::Angle { points, count } = measurement.geometry {
        return [
            SelectionHandle::AngleVertex,
            SelectionHandle::AngleFirstArm,
            SelectionHandle::AngleSecondArm,
        ]
        .into_iter()
        .zip(points)
        .take(usize::from(count.min(3)))
        .map(|(handle, point)| (point, handle))
        .collect();
    }
    let Some(bounds) = measurement.unrotated_bounds() else {
        return Vec::new();
    };
    let RulerGeometry::Length { rotation, .. } = measurement.geometry else {
        return Vec::new();
    };
    let center = rect_center(bounds);
    frame_handles(bounds)
        .into_iter()
        .filter(|(_, handle)| match handle {
            SelectionHandle::Top | SelectionHandle::Bottom => {
                bounds.width() > tolerance * 3.0 && bounds.height() > tolerance * 2.0
            }
            SelectionHandle::Left | SelectionHandle::Right => {
                bounds.height() > tolerance * 3.0 && bounds.width() > tolerance * 2.0
            }
            _ => true,
        })
        .map(|(point, handle)| (rotated(point, center, rotation), handle))
        .collect()
}

/// 【尺子】【旋转把手】根据 measurement 与 width 返回旋转把手位置；长度把手随测量框一起旋转
pub fn rotation_handle(measurement: Measurement, width: f32) -> Option<Point> {
    let bounds = measurement.unrotated_bounds()?;
    let center = rect_center(bounds);
    let unit = measurement.unit(width);
    let clearance = if measurement.mode() == RulerMode::Angle {
        ANGLE_FRAME_PADDING * measurement.style_scale
    } else {
        32.0 * unit
    };
    let point = (
        center.0,
        bounds.top() - clearance - 20.0 * measurement.style_scale,
    );
    let rotation = match measurement.geometry {
        RulerGeometry::Length { rotation, .. } => rotation,
        _ => 0.0,
    };
    Some(rotated(point, center, rotation))
}

/// 【尺子】【线段距离】返回 point 到 start、end 线段的最短距离；零长度线段退化为点
fn segment_distance(point: Point, start: Point, end: Point) -> f32 {
    let delta = (end.0 - start.0, end.1 - start.1);
    let square = delta.0 * delta.0 + delta.1 * delta.1;
    if square <= 0.0 {
        return distance(point, start);
    }
    let t =
        (((point.0 - start.0) * delta.0 + (point.1 - start.1) * delta.1) / square).clamp(0.0, 1.0);
    distance(point, (start.0 + delta.0 * t, start.1 + delta.1 * t))
}

/// 【尺子】【最近控制点】在 handles 中查找 point 的最近命中；tolerance 为命中距离，未命中返回 None
fn nearest(
    handles: impl IntoIterator<Item = (Point, SelectionHandle)>,
    point: Point,
    tolerance: f32,
) -> SelectionHandle {
    handles
        .into_iter()
        .map(|(position, handle)| (distance(point, position), handle))
        .filter(|(distance, _)| *distance <= tolerance)
        .min_by(|first, second| first.0.total_cmp(&second.0))
        .map_or(SelectionHandle::None, |(_, handle)| handle)
}

/// 【尺子】【测量命中】根据 measurement、point、width 和 selected 查找控制点；interior_moves 决定长度框内部能否整体移动
pub fn hit_test(
    measurement: Measurement,
    point: Point,
    width: f32,
    selected: bool,
    interior_moves: bool,
) -> SelectionHandle {
    let tolerance = HANDLE_TOLERANCE * measurement.style_scale;
    if selected
        && rotation_handle(measurement, width)
            .is_some_and(|handle| distance(point, handle) <= tolerance * 1.4)
    {
        return SelectionHandle::Rotate;
    }
    let handle = nearest(handles(measurement, tolerance), point, tolerance * 1.4);
    if handle != SelectionHandle::None {
        return handle;
    }
    match measurement.geometry {
        RulerGeometry::Angle { points, count } => {
            if points[1..usize::from(count.clamp(1, 3))]
                .iter()
                .any(|end| segment_distance(point, points[0], *end) <= tolerance)
            {
                return SelectionHandle::Move;
            }
            if selected && let Some(frame) = frame(measurement) {
                let handle = nearest(frame_handles(frame), point, tolerance * 1.4);
                if handle != SelectionHandle::None {
                    return handle;
                }
                if contains(frame, point) {
                    return SelectionHandle::Move;
                }
            }
        }
        RulerGeometry::Length { rotation, .. } => {
            let Some(bounds) = measurement.unrotated_bounds() else {
                return SelectionHandle::None;
            };
            let point = rotated(point, rect_center(bounds), -rotation);
            if bounds
                .outset(tolerance, tolerance)
                .is_some_and(|expanded| contains(expanded, point))
                && (interior_moves
                    || !bounds
                        .inset(tolerance, tolerance)
                        .is_some_and(|inner| contains(inner, point)))
            {
                return SelectionHandle::Move;
            }
        }
    }
    SelectionHandle::None
}

/// 【尺子】【测量变换】根据 handle、start、point 调整 before；keep_aspect 保持角度框比例，snap 吸附边或旋转方向
pub fn dragged(
    before: Measurement,
    handle: SelectionHandle,
    start: Point,
    point: Point,
    keep_aspect: bool,
    snap: bool,
) -> Measurement {
    if !point.0.is_finite() || !point.1.is_finite() {
        return before;
    }
    match handle {
        SelectionHandle::Move => {
            before.mapped(|old| (old.0 + point.0 - start.0, old.1 + point.1 - start.1))
        }
        SelectionHandle::Rotate => rotate_drag(before, start, point, snap),
        SelectionHandle::AngleVertex
        | SelectionHandle::AngleFirstArm
        | SelectionHandle::AngleSecondArm => point_drag(before, handle, point, snap),
        SelectionHandle::None => before,
        _ => match before.geometry {
            RulerGeometry::Length { .. } => length_resize(before, handle, point),
            RulerGeometry::Angle { .. } => angle_resize(before, handle, point, keep_aspect),
        },
    }
}

/// 【尺子】【旋转拖动】把 start 到 point 的有向转角应用到 before；snap 使长度框或角度第一边吸附到 45 度
fn rotate_drag(before: Measurement, start: Point, point: Point, snap: bool) -> Measurement {
    let Some(bounds) = before.unrotated_bounds() else {
        return before;
    };
    let center = rect_center(bounds);
    if distance(start, center) < 0.001 || distance(point, center) < 0.001 {
        return before;
    }
    let first = (start.1 - center.1).atan2(start.0 - center.0);
    let second = (point.1 - center.1).atan2(point.0 - center.0);
    let mut degrees = (second - first).to_degrees();
    if snap {
        let base = match before.geometry {
            RulerGeometry::Length { rotation, .. } => rotation,
            RulerGeometry::Angle { points, .. } => (points[1].1 - points[0].1)
                .atan2(points[1].0 - points[0].0)
                .to_degrees(),
        };
        degrees = ((base + degrees) / 45.0).round() * 45.0 - base;
    }
    before.rotated(degrees)
}

/// 【尺子】【定点拖动】仅把 before 中 handle 对应定点移动到 point；snap 只约束两条边的端点
fn point_drag(
    before: Measurement,
    handle: SelectionHandle,
    point: Point,
    snap: bool,
) -> Measurement {
    let RulerGeometry::Angle { mut points, count } = before.geometry else {
        return before;
    };
    let index = match handle {
        SelectionHandle::AngleVertex => 0,
        SelectionHandle::AngleFirstArm => 1,
        _ => 2,
    };
    points[index] = constrained_point(points[0], point, snap && index != 0);
    Measurement {
        geometry: RulerGeometry::Angle { points, count },
        ..before
    }
}

/// 【尺子】【相对锚点】根据 bounds 和 handle 返回不动锚点与两个可调整轴；非范围控制点返回空值
fn anchor(bounds: Rect, handle: SelectionHandle) -> Option<(Point, bool, bool)> {
    let center = rect_center(bounds);
    Some(match handle {
        SelectionHandle::TopLeft => ((bounds.right(), bounds.bottom()), true, true),
        SelectionHandle::TopRight => ((bounds.left(), bounds.bottom()), true, true),
        SelectionHandle::BottomLeft => ((bounds.right(), bounds.top()), true, true),
        SelectionHandle::BottomRight => ((bounds.left(), bounds.top()), true, true),
        SelectionHandle::Top => ((center.0, bounds.bottom()), false, true),
        SelectionHandle::Bottom => ((center.0, bounds.top()), false, true),
        SelectionHandle::Left => ((bounds.right(), center.1), true, false),
        SelectionHandle::Right => ((bounds.left(), center.1), true, false),
        _ => return None,
    })
}

/// 【尺子】【长度调整】将 before 的 handle 移动到 point；旋转后保持对角或对边的物理位置不变
fn length_resize(before: Measurement, handle: SelectionHandle, point: Point) -> Measurement {
    let Some(bounds) = before.unrotated_bounds() else {
        return before;
    };
    let RulerGeometry::Length { rotation, .. } = before.geometry else {
        return before;
    };
    let Some((anchor, changes_x, changes_y)) = anchor(bounds, handle) else {
        return before;
    };
    let center = rect_center(bounds);
    let mut moved = rotated(point, center, -rotation);
    if !changes_x {
        moved.0 = anchor.0;
    }
    if !changes_y {
        moved.1 = anchor.1;
    }
    let width = if changes_x {
        (moved.0 - anchor.0).abs()
    } else {
        bounds.width()
    };
    let height = if changes_y {
        (moved.1 - anchor.1).abs()
    } else {
        bounds.height()
    };
    let anchor = rotated(anchor, center, rotation);
    let moved = rotated(moved, center, rotation);
    let center = ((anchor.0 + moved.0) / 2.0, (anchor.1 + moved.1) / 2.0);
    Measurement {
        geometry: RulerGeometry::Length {
            start: (center.0 - width / 2.0, center.1 - height / 2.0),
            end: (center.0 + width / 2.0, center.1 + height / 2.0),
            rotation,
        },
        ..before
    }
}

/// 【尺子】【角度范围调整】根据 handle 与 point 映射 before 的三个定点；keep_aspect 保持夹角，共线轴不扩张
fn angle_resize(
    before: Measurement,
    handle: SelectionHandle,
    mut point: Point,
    keep_aspect: bool,
) -> Measurement {
    let Some(bounds) = before.unrotated_bounds() else {
        return before;
    };
    let Some((anchor, changes_x, changes_y)) = anchor(bounds, handle) else {
        return before;
    };
    let padding = ANGLE_FRAME_PADDING * before.style_scale;
    if changes_x {
        point.0 += if anchor.0 == bounds.right() {
            padding
        } else {
            -padding
        };
    }
    if changes_y {
        point.1 += if anchor.1 == bounds.bottom() {
            padding
        } else {
            -padding
        };
    }
    let original = match before.geometry {
        RulerGeometry::Angle { points, .. } => points,
        _ => return before,
    };
    let Some(raw) = points_bounds(&original, 1.0 / before.pixel_scale) else {
        return before;
    };
    let flat_x = raw.width() <= 1.0 / before.pixel_scale;
    let flat_y = raw.height() <= 1.0 / before.pixel_scale;
    let sx = if changes_x && !flat_x {
        (point.0 - anchor.0)
            / if anchor.0 == bounds.right() {
                -bounds.width()
            } else {
                bounds.width()
            }
    } else {
        1.0
    };
    let sy = if changes_y && !flat_y {
        (point.1 - anchor.1)
            / if anchor.1 == bounds.bottom() {
                -bounds.height()
            } else {
                bounds.height()
            }
    } else {
        1.0
    };
    let (sx, sy) = if keep_aspect && !flat_x && !flat_y {
        let scale = if changes_x && changes_y {
            sx.abs().max(sy.abs())
        } else if changes_x {
            sx.abs()
        } else {
            sy.abs()
        };
        (sx.signum() * scale, sy.signum() * scale)
    } else {
        (sx, sy)
    };
    before.mapped(|point| {
        (
            anchor.0 + (point.0 - anchor.0) * sx,
            anchor.1 + (point.1 - anchor.1) * sy,
        )
    })
}
