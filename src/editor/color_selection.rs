//! 【标注颜色】【应用事务】区分属性编辑与快捷色板，不把已有标注颜色写回工具默认值

use super::edits::{active_annotation_idx, damage_color_popover};
use super::{EditorState, annotation_drag::rebuild_annotation, dirty::apply_damage_rects};
use crate::types::AnnotationShape;
use crate::ui::{
    color_popover::ColorTarget,
    panel::{UiPanel, emit_panel_damage},
};
use tiny_skia::Color;

/// 【标注颜色】【颜色选用】在 state 中选择 color 并记录历史；dirty 返回控件与标注损坏区域
pub fn use_color(state: &mut EditorState, color: Color, dirty: &mut u32) {
    if state.color_popover.target == ColorTarget::Outline
        && !state.settings_panel.has_outline_swatch()
    {
        state.color_popover.target = ColorTarget::Main;
    }
    state.color_popover.select_color(color);
    state.color_popover.record_used_color(color);
    state.color_popover.refresh_palette();
    state.color_popover.sync_field_values();
    damage_color_popover(state, dirty);
    apply_color_selection(state, color, true, dirty);
}

/// 【标注颜色】【属性应用】按 state 的颜色目标应用 color；record_undo 控制独立操作，dirty 返回重绘区域
pub fn apply_color_selection(
    state: &mut EditorState,
    color: Color,
    record_undo: bool,
    dirty: &mut u32,
) {
    apply_selection(state, color, record_undo, true, dirty);
}

/// 【标注颜色】【透明度应用】将 color 写入 state 属性，record_undo 控制标注历史，dirty 返回重绘范围；不重新设置文档格式，无返回值
pub fn apply_color_opacity(
    state: &mut EditorState,
    color: Color,
    record_undo: bool,
    dirty: &mut u32,
) {
    apply_selection(state, color, record_undo, false, dirty);
}

/// 【标注颜色】【来源分派】按 state、color、record_undo 应用属性，reformat 区分取色与透明度，dirty 返回损坏范围，无返回值
fn apply_selection(
    state: &mut EditorState,
    color: Color,
    record_undo: bool,
    reformat: bool,
    dirty: &mut u32,
) {
    if super::annotation_collection::properties::active(state) {
        super::annotation_collection::properties::set_color(
            state,
            color,
            !reformat,
            record_undo,
            dirty,
        );
        refresh(state, dirty);
        return;
    }
    let target = state.color_popover.target;
    let index = active_annotation_idx(state);
    let detailed = state.color_popover.open && state.color_popover.radial.is_none();
    let changed = index.is_some_and(|i| match target {
        ColorTarget::Main => state.annotations[i].color != color,
        ColorTarget::Outline => matches!(&state.annotations[i].shape,
            AnnotationShape::Text { outline, .. } if *outline != Some(color)),
        ColorTarget::TextBackground => matches!(&state.annotations[i].shape,
            AnnotationShape::Text { appearance, .. } if appearance.background != color),
    });
    // 1. 【标注颜色】【事务捕获】属性取色的一次打开共用快照，空操作不清除重做记录
    if changed {
        if detailed {
            if state.color_popover.pre_edit_snapshot.is_none() {
                state.color_popover.pre_edit_snapshot = Some(state.snapshot());
            }
        } else if record_undo {
            state.push_undo();
        }
        let i = index.unwrap();
        state.damage_rects.push(super::DamageZone::Global(
            state.annotations[i].damage_bbox(true),
        ));
        state
            .layer_damage_rects
            .push(state.annotations[i].damage_bbox(false));
        match target {
            ColorTarget::Main => state.annotations[i].color = color,
            ColorTarget::Outline => {
                if let AnnotationShape::Text { outline, .. } = &mut state.annotations[i].shape {
                    *outline = Some(color);
                }
            }
            ColorTarget::TextBackground => {
                if let AnnotationShape::Text { appearance, .. } = &mut state.annotations[i].shape {
                    appearance.background = color;
                }
            }
        }
        rebuild_annotation(state, i);
    }
    // 2. 【标注颜色】【默认隔离】只有没有属性对象时才修改工具默认色，草稿仍跟随当前输入
    if index.is_none() {
        match target {
            ColorTarget::Main => state.tool_settings.color = color,
            ColorTarget::Outline => {
                state.tool_settings.outline = true;
                state.tool_settings.outline_color = color;
            }
            ColorTarget::TextBackground => state.tool_settings.text_background = color,
        }
    }
    if target == ColorTarget::Main
        && let Some(draft) = &mut state.pending
    {
        draft.color = color;
    }
    if target == ColorTarget::TextBackground
        && let Some(draft) = &mut state.pending
        && let AnnotationShape::Text { appearance, .. } = &mut draft.shape
    {
        appearance.background = color;
    }
    // 3. 【标注颜色】【文档格式】属性取色按原版重设字体，透明度滑条不能额外占用正文撤销记录
    super::text_session_format::sync(state, reformat, dirty);
    refresh(state, dirty);
}

/// 【标注颜色】【快捷应用】将 color 写入 state 默认色与选中标注；dirty 返回重绘区域，选择后由面板关闭
pub fn apply_quick_color(state: &mut EditorState, color: Color, dirty: &mut u32) {
    state.color_popover.target = ColorTarget::Main;
    let reformat = state
        .text_session
        .as_ref()
        .is_none_or(|session| session.original_id.is_none());
    apply_selection(state, color, true, reformat, dirty);
    state.tool_settings.color = color;
    state.color_popover.record_used_color(color);
    state.color_popover.select_color(color);
    state.color_popover.sync_field_values();
    damage_color_popover(state, dirty);
}

/// 【标注颜色】【属性重绘】对 state 属性面板发送损坏区域；dirty 返回受影响显示器，不改变界面几何
fn refresh(state: &mut EditorState, dirty: &mut u32) {
    state.settings_panel.dirty = true;
    if let Some(rect) = state.settings_panel.rect() {
        emit_panel_damage(
            rect,
            state.settings_panel.monitor_idx,
            &mut state.damage_rects,
            dirty,
        );
    }
    apply_damage_rects(state, dirty);
}
