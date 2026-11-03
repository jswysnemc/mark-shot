//! 【尺子】【角度定点】管理顶点、第一边与第二边，不依赖鼠标或显示服务器

use super::geometry::distance;
use super::{Measurement, Point, RulerGeometry};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DraftStep {
    Continue,
    Commit,
}

/// 【尺子】【角度跟随】将 measurement 最后一个跟随点更新为 point；返回是否发生改变
pub fn follow(measurement: &mut Measurement, point: Point) -> bool {
    if !point.0.is_finite() || !point.1.is_finite() {
        return false;
    }
    let RulerGeometry::Angle { points, count } = &mut measurement.geometry else {
        return false;
    };
    let Some(last) = count.checked_sub(1).filter(|index| *index < 3) else {
        return false;
    };
    let changed = points[usize::from(last)] != point;
    points[usize::from(last)] = point;
    changed
}

/// 【尺子】【角度松开】以 point 定下当前跟随点；minimum_arm 是逻辑坐标最短边，返回继续定点或可提交状态
pub fn release(measurement: &mut Measurement, point: Point, minimum_arm: f32) -> DraftStep {
    follow(measurement, point);
    let RulerGeometry::Angle { points, count } = &mut measurement.geometry else {
        return DraftStep::Continue;
    };
    if *count < 2 || *count > 3 {
        return DraftStep::Continue;
    }
    let last = usize::from(*count - 1);
    // 1. 【尺子】【角度松开】单击或过短手势保持跟随状态，不产生退化夹角
    if distance(points[0], points[last]) < minimum_arm
        || distance(points[last - 1], points[last]) < minimum_arm
    {
        return DraftStep::Continue;
    }
    // 2. 【尺子】【角度松开】第一边确认后追加第二边，三点完整才允许提交
    if *count == 2 {
        points[2] = points[1];
        *count = 3;
        return DraftStep::Continue;
    }
    DraftStep::Commit
}

/// 【尺子】【角度吸附】按 snap 将 vertex 到 point 的方向吸附到 45 度倍数；返回保持边长的终点
pub fn constrained_point(vertex: Point, point: Point, snap: bool) -> Point {
    if !snap {
        return point;
    }
    let delta = (point.0 - vertex.0, point.1 - vertex.1);
    let angle = (delta.1.atan2(delta.0) / std::f32::consts::FRAC_PI_4).round()
        * std::f32::consts::FRAC_PI_4;
    let length = delta.0.hypot(delta.1);
    (
        vertex.0 + angle.cos() * length,
        vertex.1 + angle.sin() * length,
    )
}
