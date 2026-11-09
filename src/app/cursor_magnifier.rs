//! 【光标放大区】【显隐损坏】业务输入完成后同步显示阶段，旧圆环和取色说明全部进入损坏范围

use crate::{
    editor::EditorState,
    ui::{
        magnifier::{MagnifierState, magnifier_rect},
        panel::emit_panel_damage,
    },
};

/// 【光标放大区】【旧像素恢复】为 state 中 point 的像素圆镜和读数记录完整描边损坏，dirty 返回所属显示器，无返回值
fn damage(state: &mut EditorState, point: Option<MagnifierState>, dirty: &mut u32) {
    let Some(point) = point else { return };
    let size = state.placements[point.monitor_idx].size;
    let rect = magnifier_rect(
        (point.pos.0 as f32, point.pos.1 as f32),
        size.0 as f32,
        size.1 as f32,
    );
    let pad = crate::config::get().magnifier.outline_width / 2.0 + 1.0;
    let Some(rect) = tiny_skia::Rect::from_ltrb(
        rect.left() - pad,
        rect.top() - pad,
        rect.right() + pad,
        rect.bottom() + pad,
    ) else {
        return;
    };
    emit_panel_damage(rect, point.monitor_idx, &mut state.damage_rects, dirty);
}

/// 【光标放大区】【阶段同步】在 state 业务事件完成后比较显隐，dirty 返回旧、新范围；无变化不增加帧或定时器
pub fn sync(state: &mut EditorState, dirty: &mut u32) {
    let shown = state.magnifier_shown();
    if shown == state.magnifier.visible {
        return;
    }
    if state.magnifier.visible {
        damage(
            state,
            state.magnifier.prev.or(state.magnifier.current),
            dirty,
        );
        damage(state, state.magnifier.current, dirty);
    }
    if shown {
        damage(state, state.magnifier.current, dirty);
    }
    state.magnifier.visible = shown;
}
