//! 【放大镜】【双框拖动】处理独立平移与倍率不变的联动缩放，不管理撤销或图像缓冲

use tiny_skia::Rect;

use super::geometry::{centered_rect, finite};
use super::model::MIN_DIAMETER;
use super::{Magnifier, MagnifierShape, Point};
use crate::types::SelectionHandle;

/// 【放大镜】【源框缩放识别】返回 handle 是否属于源框八个缩放控制点，不包括源框内部平移
pub fn is_source_resize(handle: SelectionHandle) -> bool {
    matches!(
        handle,
        SelectionHandle::MagnifierSourceTopLeft
            | SelectionHandle::MagnifierSourceTop
            | SelectionHandle::MagnifierSourceTopRight
            | SelectionHandle::MagnifierSourceLeft
            | SelectionHandle::MagnifierSourceRight
            | SelectionHandle::MagnifierSourceBottomLeft
            | SelectionHandle::MagnifierSourceBottom
            | SelectionHandle::MagnifierSourceBottomRight
    )
}

/// 【放大镜】【源框控制点】把 handle 的通用矩形方向映射到源框专用方向；返回专用控制点
fn source_handle(handle: SelectionHandle) -> SelectionHandle {
    match handle {
        SelectionHandle::TopLeft => SelectionHandle::MagnifierSourceTopLeft,
        SelectionHandle::Top => SelectionHandle::MagnifierSourceTop,
        SelectionHandle::TopRight => SelectionHandle::MagnifierSourceTopRight,
        SelectionHandle::Left => SelectionHandle::MagnifierSourceLeft,
        SelectionHandle::Right => SelectionHandle::MagnifierSourceRight,
        SelectionHandle::BottomLeft => SelectionHandle::MagnifierSourceBottomLeft,
        SelectionHandle::Bottom => SelectionHandle::MagnifierSourceBottom,
        SelectionHandle::BottomRight => SelectionHandle::MagnifierSourceBottomRight,
        _ => handle,
    }
}

/// 【放大镜】【单框控制点】返回 rect、shape 对应的未旋转控制点；矩形角点优先，圆形仅使用上下左右
fn frame_controls(rect: Rect, shape: MagnifierShape) -> Vec<(SelectionHandle, Point)> {
    let center = (
        (rect.left() + rect.right()) / 2.0,
        (rect.top() + rect.bottom()) / 2.0,
    );
    let mut controls = Vec::with_capacity(8);
    if shape == MagnifierShape::Rectangle {
        controls.extend([
            (SelectionHandle::TopLeft, (rect.left(), rect.top())),
            (SelectionHandle::TopRight, (rect.right(), rect.top())),
            (SelectionHandle::BottomLeft, (rect.left(), rect.bottom())),
            (SelectionHandle::BottomRight, (rect.right(), rect.bottom())),
        ]);
    }
    controls.extend([
        (SelectionHandle::Top, (center.0, rect.top())),
        (SelectionHandle::Bottom, (center.0, rect.bottom())),
        (SelectionHandle::Left, (rect.left(), center.1)),
        (SelectionHandle::Right, (rect.right(), center.1)),
    ]);
    controls
}

/// 【放大镜】【完整控制点】返回 magnifier 的镜框、源框控制点及旋转后的坐标；空草稿返回空列表
pub fn controls(magnifier: Magnifier) -> Vec<(SelectionHandle, Point)> {
    let (Some(lens), Some(source)) = (magnifier.lens_rect(), magnifier.source_rect()) else {
        return Vec::new();
    };
    let mut points = frame_controls(lens, magnifier.shape);
    points.extend(
        frame_controls(source, magnifier.shape)
            .into_iter()
            .map(|(handle, point)| (source_handle(handle), point)),
    );
    for (_, point) in &mut points {
        *point = magnifier.to_visual(*point);
    }
    points
}

/// 【放大镜】【旋转把手】返回 magnifier 联合框上方的独立旋转控制点；空草稿返回空值
pub fn rotation_handle(magnifier: Magnifier) -> Option<Point> {
    let bounds = magnifier.unrotated_bounds()?;
    Some(magnifier.to_visual((
        (bounds.left() + bounds.right()) / 2.0,
        bounds.top() - 32.0 * magnifier.style_scale,
    )))
}

/// 【放大镜】【单框命中】返回 point 是否位于 rect、shape 对应的图形内；tolerance 提供可操作边界留量
fn contains(rect: Rect, shape: MagnifierShape, point: Point, tolerance: f32) -> bool {
    if shape == MagnifierShape::Rectangle {
        return point.0 >= rect.left() - tolerance
            && point.0 <= rect.right() + tolerance
            && point.1 >= rect.top() - tolerance
            && point.1 <= rect.bottom() + tolerance;
    }
    let center = (
        (rect.left() + rect.right()) / 2.0,
        (rect.top() + rect.bottom()) / 2.0,
    );
    let x = (point.0 - center.0) / (rect.width() / 2.0 + tolerance);
    let y = (point.1 - center.1) / (rect.height() / 2.0 + tolerance);
    x * x + y * y <= 1.0
}

/// 【放大镜】【操作命中】以 point 命中 magnifier；selected 时先检查镜框和源框控制点，再按源框优先命中内部，返回操作方式
pub fn hit_test(magnifier: Magnifier, point: Point, selected: bool) -> SelectionHandle {
    if !finite(point) {
        return SelectionHandle::None;
    }
    let (Some(lens), Some(source)) = (magnifier.lens_rect(), magnifier.source_rect()) else {
        return SelectionHandle::None;
    };
    let tolerance =
        (crate::config::get().input.handle_hit_width / 2.0).max(1.0) * magnifier.style_scale;
    let close = |control: Point| (control.0 - point.0).hypot(control.1 - point.1) <= tolerance;
    if selected {
        if let Some((handle, _)) = controls(magnifier)
            .into_iter()
            .find(|(_, control)| close(*control))
        {
            return handle;
        }
        if rotation_handle(magnifier).is_some_and(close) {
            return SelectionHandle::Rotate;
        }
    }
    let point = magnifier.to_local(point);
    if contains(source, magnifier.shape, point, tolerance) {
        SelectionHandle::MagnifierSource
    } else if contains(lens, magnifier.shape, point, tolerance) {
        SelectionHandle::MagnifierLens
    } else {
        SelectionHandle::None
    }
}

/// 【放大镜】【矩形调整】按 handle、point 调整 before，minimum 指定最小宽高；keep_aspect 保留长宽比，返回新矩形
fn resized_rect(
    before: Rect,
    handle: SelectionHandle,
    point: Point,
    minimum: Point,
    keep_aspect: bool,
) -> Rect {
    let center = (
        (before.left() + before.right()) / 2.0,
        (before.top() + before.bottom()) / 2.0,
    );
    let (anchor, direction) = match handle {
        SelectionHandle::TopLeft => ((before.right(), before.bottom()), (-1.0, -1.0)),
        SelectionHandle::TopRight => ((before.left(), before.bottom()), (1.0, -1.0)),
        SelectionHandle::BottomLeft => ((before.right(), before.top()), (-1.0, 1.0)),
        SelectionHandle::BottomRight => ((before.left(), before.top()), (1.0, 1.0)),
        SelectionHandle::Left => ((before.right(), center.1), (-1.0, 0.0)),
        SelectionHandle::Right => ((before.left(), center.1), (1.0, 0.0)),
        SelectionHandle::Top => ((center.0, before.bottom()), (0.0, -1.0)),
        SelectionHandle::Bottom => ((center.0, before.top()), (0.0, 1.0)),
        _ => return before,
    };
    let mut width = if direction.0 == 0.0 {
        before.width()
    } else {
        ((point.0 - anchor.0) * direction.0).max(minimum.0)
    };
    let mut height = if direction.1 == 0.0 {
        before.height()
    } else {
        ((point.1 - anchor.1) * direction.1).max(minimum.1)
    };
    if keep_aspect {
        let ratio = before.width() / before.height();
        if direction.0 == 0.0 {
            width = height * ratio;
        } else if direction.1 == 0.0 {
            height = width / ratio;
        } else {
            width = width.max(height * ratio);
            height = width / ratio;
        }
    }
    let center = (
        anchor.0 + direction.0 * width / 2.0,
        anchor.1 + direction.1 * height / 2.0,
    );
    centered_rect(center, (width, height)).unwrap_or(before)
}

/// 【放大镜】【等比尺寸限制】将 size 限制到 bounds；keep_aspect 时同比缩小，返回可容纳的宽高
fn fitted_size(size: Point, bounds: Rect, keep_aspect: bool) -> Point {
    if !keep_aspect {
        return (size.0.min(bounds.width()), size.1.min(bounds.height()));
    }
    let factor = (bounds.width() / size.0)
        .min(bounds.height() / size.1)
        .min(1.0);
    (size.0 * factor, size.1 * factor)
}

/// 【放大镜】【拖动几何】按 handle、start、point 更新 before；Ctrl 保持矩形比例，Shift 吸附旋转，返回新标注，无效输入保持原值
pub fn dragged(
    before: Magnifier,
    handle: SelectionHandle,
    start: Point,
    point: Point,
    ctrl: bool,
    shift: bool,
) -> Magnifier {
    if !finite(start) || !finite(point) || !before.measures() {
        return before;
    }
    if handle == SelectionHandle::Rotate {
        let from = (start.0 - before.pivot.0, start.1 - before.pivot.1);
        let to = (point.0 - before.pivot.0, point.1 - before.pivot.1);
        if from.0.hypot(from.1) < 1.0 || to.0.hypot(to.1) < 1.0 {
            return before;
        }
        let rotation = before.rotation + to.1.atan2(to.0) - from.1.atan2(from.0);
        let rotation = if shift {
            (rotation / std::f32::consts::FRAC_PI_4).round() * std::f32::consts::FRAC_PI_4
        } else {
            rotation
        };
        return Magnifier {
            rotation: (rotation + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI,
            ..before
        };
    }
    if handle == SelectionHandle::Move {
        let Some(bounds) = before.bounds() else {
            return before;
        };
        let axis = |delta: f32, min: f32, max: f32| {
            if min <= max {
                delta.clamp(min, max)
            } else {
                0.0
            }
        };
        return before.translated((
            axis(
                point.0 - start.0,
                before.image_bounds.left() - bounds.left(),
                before.image_bounds.right() - bounds.right(),
            ),
            axis(
                point.1 - start.1,
                before.image_bounds.top() - bounds.top(),
                before.image_bounds.bottom() - bounds.bottom(),
            ),
        ));
    }
    let (start, point) = (before.to_local(start), before.to_local(point));
    let delta = (point.0 - start.0, point.1 - start.1);
    let mut result = before;
    match handle {
        SelectionHandle::MagnifierSource => {
            result.source = (before.source.0 + delta.0, before.source.1 + delta.1)
        }
        SelectionHandle::MagnifierLens => {
            result.center = (before.center.0 + delta.0, before.center.1 + delta.1)
        }
        _ => {
            let source = is_source_resize(handle);
            let Some(rect) = (if source {
                before.source_rect()
            } else {
                before.lens_rect()
            }) else {
                return before;
            };
            let minimum = (MIN_DIAMETER / before.pixel_scale)
                .min(before.image_bounds.width())
                .min(before.image_bounds.height());
            let minimum = if source {
                minimum / before.zoom
            } else {
                minimum
            };
            let keep_aspect = before.shape == MagnifierShape::Circle || ctrl;
            let rect = resized_rect(
                rect,
                handle.generic_rect_handle(),
                point,
                (minimum, minimum),
                keep_aspect,
            );
            let requested = (rect.width(), rect.height());
            if source {
                // 1. 【放大镜】【源框缩放】先按倍率计算镜框，再限制镜框尺寸，源框使用实际镜框尺寸反算
                result.size = fitted_size(
                    (requested.0 * before.zoom, requested.1 * before.zoom),
                    before.image_bounds,
                    keep_aspect,
                );
                result.source = (
                    (rect.left() + rect.right()) / 2.0,
                    (rect.top() + rect.bottom()) / 2.0,
                );
            } else {
                // 2. 【放大镜】【镜框缩放】源框中心保持不变，仅在原图边缘限制完整源框
                result.size = fitted_size(requested, before.image_bounds, keep_aspect);
                result.center = (
                    (rect.left() + rect.right()) / 2.0,
                    (rect.top() + rect.bottom()) / 2.0,
                );
            }
        }
    }
    result.constrained()
}
