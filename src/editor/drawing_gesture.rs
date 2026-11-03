//! 【标注】【绘图取消】取消未提交图形，并清理已经缓存的画笔片段

use crate::editor::dirty::apply_damage_rects;
use crate::editor::{DamageZone, EditorState};

/// 【标注】【取消绘图】清除 state 的未提交绘图及缓存片段；dirty_mask 返回重绘显示器，不处理已有标注的拖动
pub fn cancel(state: &mut EditorState, dirty_mask: &mut u32) {
    if state.ann_drag.is_some() {
        return;
    }

    // 1. 【标注】【取消绘图】清除屏幕草稿，未缓存笔迹不触及持久图层
    if let Some(annotation) = state.pending.take().or_else(|| state.prev_pending.take()) {
        state.filters.remove(annotation.id);
        state
            .damage_rects
            .push(DamageZone::Global(annotation.damage_bbox(false)));
        if state.pending_pen_baked != 0 {
            // 2. 【标注】【取消绘图】恢复缓存前的原始像素，不重新绘制已有标注
            state.restore_pen_segments(dirty_mask);
        }
    }

    // 3. 【标注】【取消绘图】结束输入手势，后续工具不继承旧草稿或鼠标状态
    state.prev_pending = None;
    state.pending_pen_baked = 0;
    state.input.mouse_down = false;
    state.input.drag_start = None;
    state.input.straight = false;
    state.tool_active = false;
    apply_damage_rects(state, dirty_mask);
}
