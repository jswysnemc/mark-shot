//! 【尺子】【平移吸附】以尺子的可见几何对齐识别区域，不改变长度、角度或拖动抓点

use super::snap_targets::{SnapAxes, Targets};
use crate::{
    editor::EditorState,
    features::ruler::{Measurement, Point, RulerGeometry, simple_frame},
};

/// 【尺子】【平移修正】按原测量、按下位置和当前指针返回修正后的拖动坐标，不累积上一帧位移
pub(crate) fn pointer(
    state: &mut EditorState,
    before: Measurement,
    start: Point,
    current: Point,
) -> Point {
    if current == start {
        return current;
    }
    let Some(targets) = Targets::for_ruler(state, current) else {
        return current;
    };
    let delta = (current.0 - start.0, current.1 - start.1);
    let mut axes = SnapAxes::default();
    let moved = |anchor: Point| (anchor.0 + delta.0, anchor.1 + delta.1);
    let mut inspect = |projection, area| axes.consider(projection, area);
    // 1. 【尺子】【锚点选择】测量定点负责边角吸附，整条平行尺边可与较小控件的边界对齐
    if let Some(handles) = simple_frame::handles(before) {
        for (tip, _) in handles {
            targets.visit(moved(tip), &mut inspect);
        }
        targets.visit_edge(moved(handles[0].0), moved(handles[1].0), &mut inspect);
    } else if let RulerGeometry::Angle { points, count } = before.geometry {
        for (index, &point) in points[..usize::from(count.min(3))].iter().enumerate() {
            targets.visit(moved(point), &mut inspect);
            if index > 0 {
                targets.visit_edge(moved(points[0]), moved(point), &mut inspect);
            }
        }
    } else if let Some(corners) = before.corners() {
        for (index, point) in corners.into_iter().enumerate() {
            targets.visit(moved(point), &mut inspect);
            targets.visit_edge(moved(point), moved(corners[(index + 1) % 4]), &mut inspect);
        }
    }
    axes.projection().apply(current)
}
