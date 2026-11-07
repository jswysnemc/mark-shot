//! 【文字滚轮】【输入分派】控件使用角度刻度，画布使用一点五点增量与三百五十毫秒连续历史

use crate::{
    editor::{EditorState, edits::active_annotation_idx, text_edits, text_size},
    types::{AnnotationShape, ScrollEvent},
    ui::text_properties::layout::Control,
};
use std::time::Instant;

/// 【文字滚轮】【滑条输入】把 event 应用到 state 指针下的字号或透明度滑条，dirty 返回重绘范围，返回是否消费
pub fn properties(state: &mut EditorState, event: ScrollEvent, dirty: &mut u32) -> bool {
    let Some(control @ (Control::Size | Control::Opacity)) = super::properties::hit(state) else {
        return false;
    };
    state.commit_text_size_wheel();
    state
        .input
        .text_wheels
        .prepare(state.settings_panel.active_source);
    let input = if control == Control::Size {
        &mut state.input.text_wheels.size
    } else {
        &mut state.input.text_wheels.opacity
    };
    let steps = input.slider(event.angle_y(), state.input.ctrl || state.input.shift, 10);
    let (current, range) = super::properties::value(state, control);
    let new_value = (current + steps).clamp(range.0, range.1);
    if steps != 0 && new_value == current {
        if control == Control::Size {
            state.input.text_wheels.size.reset();
        } else {
            state.input.text_wheels.opacity.reset();
        }
    }
    if steps != 0 {
        super::properties::apply(state, control, new_value, dirty);
    }
    true
}

/// 【文字滚轮】【画布输入】按 event 和 now 调整 state 当前文字，dirty 返回重绘范围；默认值变化不入历史，拖动时不修改
pub fn canvas(state: &mut EditorState, event: ScrollEvent, now: Instant, dirty: &mut u32) {
    if crate::editor::annotation_collection::properties::active(state) {
        if crate::editor::annotation_collection::properties::canvas_wheel(state, event, now, dirty)
        {
            super::super::settings::update_settings_panel(state, dirty);
        }
        return;
    }
    if state.tool_active
        || state.input.mouse_down
        || state.ann_drag.is_some()
        || state.toolbar_drag.is_some()
    {
        return;
    }
    let index = active_annotation_idx(state).filter(|index| {
        matches!(
            state.annotations[*index].shape,
            AnnotationShape::Text { .. }
        )
    });
    let context = index.map(|index| state.annotations[index].id);
    state.prepare_text_size_wheel(context, now);
    let steps = state.text_size_history.input.canvas(event.canvas_steps());
    if steps == 0 {
        return;
    }
    let snapshot = index.map(|_| state.snapshot());
    let points = text_edits::current_points(state) + steps as f32;
    if text_size::set(state, points, false, dirty) {
        if let Some(snapshot) = snapshot {
            state.queue_text_size_wheel(snapshot, now);
        }
        super::super::settings::update_settings_panel(state, dirty);
    }
}
