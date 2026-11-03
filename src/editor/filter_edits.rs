//! 【滤镜】【属性事务】模式只修改当前对象，强度沿用原版同步工具默认值的规则

use crate::editor::{
    DamageZone, EditorState,
    annotation_drag::rebuild_annotation,
    dirty::apply_damage_rects,
    edits::{active_annotation_idx, commit_settings_change},
};
use crate::features::filter::{FilterMode, clamped_strength};
use crate::tools::Tool;
use crate::types::AnnotationShape;

/// 【滤镜】【属性可用】返回 state 当前工具或选中对象是否支持滤镜属性，不在已有区域拖动中修改样式
pub fn available(state: &EditorState) -> bool {
    state.ann_drag.is_none()
        && match active_annotation_idx(state).and_then(|index| state.annotations.get(index)) {
            Some(annotation) => matches!(annotation.shape, AnnotationShape::Redact { .. }),
            None => state.selected_tool == Tool::Redact,
        }
}

/// 【滤镜】【当前模式】返回 state 当前对象的模式；没有选中滤镜时读取后续默认模式
pub fn current_mode(state: &EditorState) -> FilterMode {
    match active_annotation_idx(state)
        .and_then(|index| state.annotations.get(index))
        .map(|annotation| &annotation.shape)
    {
        Some(AnnotationShape::Redact { filter }) => filter.mode,
        _ => state.tool_settings.filter_mode,
    }
}

/// 【滤镜】【当前强度】返回 state 当前对象或工具的原图强度，不按预览比例换算
pub fn current_strength(state: &EditorState) -> f32 {
    match active_annotation_idx(state)
        .and_then(|index| state.annotations.get(index))
        .map(|annotation| &annotation.shape)
    {
        Some(AnnotationShape::Redact { filter }) => filter.strength,
        _ => state.tool_settings.strength,
    }
}

/// 【滤镜】【模式修改】把 mode 用于 state 选中区域或后续默认值；dirty_mask 返回损坏范围，重复选择不产生历史
pub fn set_mode(state: &mut EditorState, mode: FilterMode, dirty_mask: &mut u32) {
    if !available(state) || current_mode(state) == mode {
        return;
    }
    if let Some(index) = active_annotation_idx(state) {
        let annotation = &state.annotations[index];
        state
            .damage_rects
            .push(DamageZone::Global(annotation.damage_bbox(true)));
        state.layer_damage_rects.push(annotation.damage_bbox(false));
        state.push_undo();
        if let AnnotationShape::Redact { filter } = &mut state.annotations[index].shape {
            filter.mode = mode;
        }
        rebuild_annotation(state, index);
    } else {
        state.tool_settings.filter_mode = mode;
        if let Some(annotation) = state.pending.as_mut()
            && let AnnotationShape::Redact { filter } = &mut annotation.shape
        {
            filter.mode = mode;
            state
                .damage_rects
                .push(DamageZone::Global(annotation.damage_bbox(false)));
        }
    }
    state.settings_panel.dirty = true;
    apply_damage_rects(state, dirty_mask);
}

/// 【滤镜】【强度修改】把 strength 限制到原版范围并写入 state；record_undo 控制历史，dirty_mask 返回范围，无返回值
pub fn set_strength(
    state: &mut EditorState,
    strength: f32,
    record_undo: bool,
    dirty_mask: &mut u32,
) {
    if !available(state) {
        return;
    }
    let strength = clamped_strength(strength);
    commit_settings_change(
        state,
        current_strength(state) != strength,
        record_undo,
        |settings| settings.strength = strength,
        |annotation| {
            if let AnnotationShape::Redact { filter } = &mut annotation.shape {
                filter.strength = strength;
            }
        },
        dirty_mask,
    );
    if let Some(annotation) = state.pending.as_mut()
        && let AnnotationShape::Redact { filter } = &mut annotation.shape
    {
        filter.strength = strength;
        state
            .damage_rects
            .push(DamageZone::Global(annotation.damage_bbox(false)));
    }
}
