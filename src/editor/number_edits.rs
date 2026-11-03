//! 【序号】【属性事务】修改独立序号样式、重置计数并同步延迟加载的中文字形

use crate::editor::edits::{active_annotation_idx, commit_settings_change};
use crate::editor::{DamageZone, EditorState};
use crate::features::number::NumberStyle;
use crate::tools::Tool;
use crate::types::AnnotationShape;

/// 【序号】【当前样式】返回 state 中选中序号的样式；未选中序号时返回后续绘图的默认样式
pub fn current_style(state: &EditorState) -> NumberStyle {
    match active_annotation_idx(state)
        .and_then(|index| state.annotations.get(index))
        .map(|annotation| &annotation.shape)
    {
        Some(AnnotationShape::Number { mark }) => mark.style,
        _ => state.tool_settings.number_style,
    }
}

/// 【序号】【属性来源】返回 state 是否使用独立序号工具或选中独立序号，不将旧编号箭头视为同一工具
pub fn available(state: &EditorState) -> bool {
    state.selected_tool == Tool::Number
        || active_annotation_idx(state)
            .and_then(|index| state.annotations.get(index))
            .is_some_and(|annotation| matches!(annotation.shape, AnnotationShape::Number { .. }))
}

/// 【序号】【样式修改】把 style 用于 state 的选中序号或后续绘图；已有序号变化记录一次撤销，dirty_mask 返回重绘显示器
pub fn set_style(state: &mut EditorState, style: NumberStyle, dirty_mask: &mut u32) {
    if !available(state) {
        return;
    }
    let selected = active_annotation_idx(state)
        .and_then(|index| state.annotations.get(index))
        .is_some_and(|annotation| matches!(annotation.shape, AnnotationShape::Number { .. }));
    let changed = current_style(state) != style;
    commit_settings_change(
        state,
        changed,
        selected,
        |settings| {
            if !selected {
                settings.number_style = style;
            }
        },
        |annotation| {
            if let AnnotationShape::Number { mark } = &mut annotation.shape {
                mark.style = style;
            }
        },
        dirty_mask,
    );
    if let Some(annotation) = state.pending.as_mut()
        && let AnnotationShape::Number { mark } = &mut annotation.shape
        && mark.style != style
    {
        mark.style = style;
        state
            .damage_rects
            .push(DamageZone::Global(annotation.damage_bbox(false)));
        *dirty_mask = u32::MAX;
    }
}

/// 【序号】【计数重置】在 state 中把下一编号重置为一，保留已有序号；计数变化记录统一撤销，dirty_mask 返回属性重绘
pub fn reset_sequence(state: &mut EditorState, dirty_mask: &mut u32) {
    if !available(state) || state.number_sequence.next() == Some(1) {
        return;
    }
    let snapshot = state.settings_panel.pre_edit_snapshot.take();
    state.commit_snapshot(snapshot);
    let snapshot = state.color_popover.pre_edit_snapshot.take();
    state.commit_snapshot(snapshot);
    state.push_undo();
    state.number_sequence.reset();
    if let Some(annotation) = state.pending.as_mut()
        && let AnnotationShape::Number { mark } = &mut annotation.shape
    {
        mark.number = 1;
        state
            .damage_rects
            .push(DamageZone::Global(annotation.damage_bbox(false)));
    }
    state.settings_panel.dirty = true;
    *dirty_mask = u32::MAX;
}
