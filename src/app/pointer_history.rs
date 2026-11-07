//! 【截图输入】【自由轨迹】在一次事件分派中消费真实采样，帧提交仍沿用显示器节奏

use crate::{editor::EditorState, tools::Tool, types::pointer_motion::PointerMotion};

/// 【截图输入】【轨迹分派】按state、history和最终monitor/x/y处理移动，dirty返回重绘范围，无返回值
pub(super) fn dispatch(
    state: &mut EditorState,
    history: &[PointerMotion],
    monitor: usize,
    x: f64,
    y: f64,
    dirty: &mut u32,
) {
    // 1. 【截图输入】【真实自由轨迹】仅自由绘制消费中间采样，普通悬停和几何工具保持最终位置分派，所有采样仍只触发一次帧处理
    if state.input.left_canvas
        && state.pending.is_some()
        && matches!(state.selected_tool, Tool::Pen | Tool::Highlighter)
    {
        for (index, sample) in history.iter().enumerate() {
            let point = sample.min;
            if index + 1 == history.len() && (sample.monitor_idx, point) == (monitor, (x, y)) {
                continue;
            }
            super::input::handle_pointer_move(state, sample.monitor_idx, point.0, point.1, dirty);
        }
    }
    super::input::handle_pointer_move(state, monitor, x, y, dirty);
}

#[cfg(test)]
#[path = "pointer_history_tests.rs"]
mod tests;
