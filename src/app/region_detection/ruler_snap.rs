//! 【尺子】【定点吸附】按两轴独立对齐可见识别边界

use super::snap_targets::{SnapAxes, Targets};
use crate::{editor::EditorState, features::ruler::Point};

/// 【尺子】【测量定点】按 state 和画布点 point 返回吸附后的坐标，没有候选的轴沿用原坐标
pub(crate) fn point(state: &mut EditorState, point: Point) -> Point {
    let Some(targets) = Targets::for_ruler(state, point) else {
        return point;
    };
    let mut axes = SnapAxes::default();
    targets.visit(point, |projection, area| axes.consider(projection, area));
    axes.projection().apply(point)
}
