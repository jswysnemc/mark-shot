//! 【截图区域识别】【移动阈值】按真实轨迹撤销短点击，输出比例不改变五逻辑像素判定

use crate::{editor::EditorState, types::pointer_motion::PointerMotion};

/// 【截图区域识别】【实际位移】按 state 与全局逻辑 point 撤销达到五像素的候选，无返回值，撤销后不重新采纳
pub(super) fn observe(state: &mut EditorState, point: (f64, f64)) {
    let Some(press) = &mut state.region_detection.pressed else {
        return;
    };
    let distance = (point.0 - press.origin.0).hypot(point.1 - press.origin.1);
    if press.dismissed || !distance.is_finite() || distance < 5.0 {
        return;
    }
    press.dismissed = true;
    state.region_detection.cursor.reset();
    super::draw::apply(state, None);
}

/// 【截图区域识别】【合并轨迹】按 state 与 history 将各输出实际端点转换为全局坐标，保留越界后返回的撤销结果，无返回值
pub(crate) fn history(state: &mut EditorState, history: &[PointerMotion]) {
    if state.region_detection.pressed.is_none() {
        return;
    }
    for range in history {
        let Some(output) = state.placements.get(range.monitor_idx) else {
            continue;
        };
        let offset = (f64::from(output.position.0), f64::from(output.position.1));
        // 1. 【截图区域识别】【坐标统一】共线范围端点均为实际经过的位置，不组合横纵极值生成额外角点
        for point in [range.min, range.max] {
            observe(state, (offset.0 + point.0, offset.1 + point.1));
        }
    }
}
