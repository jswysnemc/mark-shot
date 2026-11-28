//! 【尺子】【箭头选框】沿箭头主轴测量，横向或纵向的选框厚度只取决于窗口单位

use super::{
    Measurement, Point, RulerGeometry, RulerStyle,
    geometry::{rect_center, rotated},
    simple_draw::axis_endpoints,
};
use crate::{features::annotation_style::window::WindowUnits, types::SelectionHandle};
use tiny_skia::Rect;

const THICKNESS: f32 = 20.0;

/// 【尺子】【箭头主轴】按测量几何返回未旋转的两个端点及水平方向，其他尺子返回空值
pub(super) fn axis(measurement: Measurement) -> Option<(Point, Point, bool)> {
    if measurement.style != RulerStyle::Simple {
        return None;
    }
    let (first, last) = axis_endpoints(measurement)?;
    Some((first, last, first.1 == last.1))
}

/// 【尺子】【固定窄框】按测量和窗口单位返回轴线居中的未旋转选框；不修改原始测量
pub(crate) fn bounds(measurement: Measurement, window: WindowUnits) -> Option<Rect> {
    let (first, last, horizontal) = axis(measurement)?;
    let half = window.canvas(THICKNESS) * 0.5;
    if horizontal {
        Rect::from_ltrb(first.0, first.1 - half, last.0, last.1 + half)
    } else {
        Rect::from_ltrb(first.0 - half, first.1, last.0 + half, last.1)
    }
}

/// 【尺子】【端点控制】按测量返回旋转后的实际箭头位置和操作方向，非箭头尺返回空值
pub(crate) fn handles(measurement: Measurement) -> Option<[(Point, SelectionHandle); 2]> {
    let (first, last, horizontal) = axis(measurement)?;
    let RulerGeometry::Length { rotation, .. } = measurement.geometry else {
        return None;
    };
    let center = rect_center(measurement.unrotated_bounds()?);
    let directions = if horizontal {
        [SelectionHandle::Left, SelectionHandle::Right]
    } else {
        [SelectionHandle::Top, SelectionHandle::Bottom]
    };
    Some([
        (rotated(first, center, rotation), directions[0]),
        (rotated(last, center, rotation), directions[1]),
    ])
}

/// 【尺子】【轴向调整】按原测量、控制点与拖动起止点调整长度，只接受沿主轴的位移，保持对端和旋转角不变
pub(super) fn resize(
    before: Measurement,
    handle: SelectionHandle,
    start: Point,
    point: Point,
) -> Option<Measurement> {
    let (first, last, horizontal) = axis(before)?;
    let RulerGeometry::Length { rotation, .. } = before.geometry else {
        return None;
    };
    let (anchor, tip) = match (horizontal, handle) {
        (true, SelectionHandle::Left) | (false, SelectionHandle::Top) => (last, first),
        (true, SelectionHandle::Right) | (false, SelectionHandle::Bottom) => (first, last),
        _ => return Some(before),
    };
    // 1. 【尺子】【轴线投影】在原选框内投影指针，避免斜拖增加厚度或改变主轴
    let center = rect_center(before.unrotated_bounds()?);
    let local = rotated(point, center, -rotation);
    let press = rotated(start, center, -rotation);
    let delta = if horizontal {
        local.0 - press.0
    } else {
        local.1 - press.1
    };
    if delta.abs() < f32::EPSILON {
        return Some(before);
    }
    let offset = (if horizontal {
        tip.0 - anchor.0
    } else {
        tip.1 - anchor.1
    }) + delta;
    let minimum = 1.0 / before.pixel_scale;
    let sign = if offset == 0.0 {
        if matches!(handle, SelectionHandle::Left | SelectionHandle::Top) {
            -1.0
        } else {
            1.0
        }
    } else {
        offset.signum()
    };
    let span = sign * offset.abs().max(minimum);
    let moved = if horizontal {
        (anchor.0 + span, anchor.1)
    } else {
        (anchor.0, anchor.1 + span)
    };
    // 2. 【尺子】【对端固定】以新中点保存主轴，使旋转后的对端仍落在原画布位置
    let middle = ((anchor.0 + moved.0) * 0.5, (anchor.1 + moved.1) * 0.5);
    let visual = rotated(middle, center, rotation);
    let shift = (visual.0 - middle.0, visual.1 - middle.1);
    Some(Measurement {
        geometry: RulerGeometry::Length {
            start: (anchor.0 + shift.0, anchor.1 + shift.1),
            end: (moved.0 + shift.0, moved.1 + shift.1),
            rotation,
        },
        ..before
    })
}
