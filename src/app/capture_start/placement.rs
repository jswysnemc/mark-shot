//! 【截图录制】【控制栏位置】根据各输出的编辑工具栏安排录制栏，位置变化同时清理旧范围

use crate::{editor::EditorState, ui::capture_start::layout};
use tiny_skia::Rect;

/// 【截图录制】【工具栏范围】按 state 与 monitor 返回当前和目标工具栏位置，动画开始时也预留最终空间
fn obstacles(state: &EditorState, monitor: usize) -> Vec<Rect> {
    if state.ui_hidden || state.selection.zone.is_none() {
        return Vec::new();
    }
    let mut bounds = Vec::new();
    for toolbar in [&state.toolbar, &state.action_toolbar] {
        if toolbar.monitor_idx != monitor || (toolbar.secondary && toolbar.opacity <= 0.0) {
            continue;
        }
        for position in [toolbar.render_pos, toolbar.position] {
            if let Some(rect) =
                Rect::from_xywh(position.0, position.1, toolbar.size.0, toolbar.size.1)
            {
                bounds.push(rect);
            }
        }
    }
    bounds
}

/// 【截图录制】【纵向位置】按 bar、obstacles 与 height 找到不遮挡工具栏的位置，优先保留顶部区域
fn top(mut bar: Rect, obstacles: &[Rect], height: i32) -> f32 {
    let limit = (height as f32 - bar.height() - 8.0).max(0.0);
    for _ in 0..=obstacles.len() {
        let Some(obstacle) = obstacles
            .iter()
            .find(|rect| crate::utils::rects_overlap(&bar, rect))
        else {
            return bar.top();
        };
        let next = obstacle.bottom() + 8.0;
        let y = if next <= limit {
            next
        } else {
            (obstacle.top() - bar.height() - 8.0).max(0.0)
        };
        bar = layout::rect(bar.left(), y, bar.width(), bar.height());
    }
    bar.top().min(limit)
}

/// 【截图录制】【位置同步】按 state 更新各显示器的录制栏位置，dirty 返回旧位置与新位置需要重绘的输出
pub(super) fn sync(state: &mut EditorState, dirty: &mut u32) {
    if state.capture_start.visible || !state.capture_start.shows_bar() {
        return;
    }
    let positions = state
        .placements
        .iter()
        .enumerate()
        .map(|(monitor, placement)| {
            let current = layout::toolbar(placement.size, &state.capture_start, monitor);
            let initial = layout::rect(current.left(), 12.0, current.width(), current.height());
            top(initial, &obstacles(state, monitor), placement.size.1)
        })
        .collect::<Vec<_>>();
    if positions == state.capture_start.bar_tops {
        return;
    }
    // 1. 【截图录制】【旧位置清理】更新坐标前标记旧栏，避免移动或隐藏编辑工具栏后留下残影
    super::damage(state, dirty);
    state.capture_start.bar_tops = positions;
    super::damage(state, dirty);
    for (monitor, top) in state.capture_start.bar_tops.iter().enumerate() {
        log::info!("【截图录制】【控制布局】monitor={monitor} top={top}");
    }
    state.capture_start.hovered =
        super::input::hit(state).map(|control| (state.input.pointer.monitor_idx, control));
}
