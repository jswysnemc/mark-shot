//! 【尺子】【拖动坐标】把选框抓点位移转换为真实测量点的边界约束和吸附

use crate::{
    app::region_detection::{ruler_move, ruler_snap},
    editor::EditorState,
    features::ruler::{Measurement, Point, drag::resize_anchor},
    tools::drawing_space::DrawingSpace,
    types::SelectionHandle,
};

/// 【尺子】【正文吸附】按原尺子 before、handle、起点 start 和当前画布坐标 current 返回拖动坐标；扩大后的外框不参与吸附
pub(super) fn pointer(
    state: &mut EditorState,
    before: Measurement,
    handle: SelectionHandle,
    start: (f64, f64),
    current: (f64, f64),
) -> Option<Point> {
    let space = DrawingSpace::from_editor(state)?;
    let press = (start.0 as f32, start.1 as f32);
    if start == current {
        return Some(press);
    }
    if let Some(tip) = resize_anchor(before, handle) {
        let target = space.clamp_point((
            f64::from(tip.0) + (current.0 - start.0),
            f64::from(tip.1) + (current.1 - start.1),
        ))?;
        let snapped = ruler_snap::point(state, target);
        return Some((press.0 + (snapped.0 - tip.0), press.1 + (snapped.1 - tip.1)));
    }
    let point = space.clamp_point(current)?;
    Some(match handle {
        SelectionHandle::Move => ruler_move::pointer(state, before, press, point),
        SelectionHandle::Rotate => (current.0 as f32, current.1 as f32),
        _ => ruler_snap::point(state, point),
    })
}
