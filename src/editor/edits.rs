pub use super::color_selection::{apply_color_selection, use_color};
// Shared stuff between color/setting changes and undo history
// Kept below `app` so tools can use it without depending on the app layer
// to keep one side relation yk

use crate::editor::annotation_drag::rebuild_annotation;
use crate::editor::dirty::{apply_damage_rects, mark_dirty};
use crate::editor::{DamageZone, EditorState};
use crate::tools::Tool;
use crate::types::{Annotation, ToolSettings};
use crate::ui::panel::{UiPanel, emit_panel_damage};

/// 【标注】【当前属性对象】返回 editor_state 中可编辑属性的选中标注索引；不返回草稿或越界索引
pub fn active_annotation_idx(editor_state: &EditorState) -> Option<usize> {
    if matches!(
        editor_state.selected_tool,
        Tool::Pick | Tool::Text | Tool::Ruler | Tool::Magnifier
    ) {
        editor_state
            .selected_annotation_index()
            .filter(|index| *index < editor_state.annotations.len())
    } else {
        None
    }
}

pub fn commit_settings_change(
    editor_state: &mut EditorState,
    changed: bool,
    record_undo: bool,
    apply_to_tool: impl FnOnce(&mut ToolSettings),
    apply_to_annotation: impl FnOnce(&mut Annotation),
    dirty_mask: &mut u32,
) {
    if !changed {
        return;
    }

    apply_to_tool(&mut editor_state.tool_settings);

    if let Some(idx) = active_annotation_idx(editor_state) {
        let old_damage = editor_state.annotations[idx].damage_bbox(true);
        let old_layer_damage = editor_state.annotations[idx].damage_bbox(false);
        editor_state
            .damage_rects
            .push(DamageZone::Global(old_damage));
        editor_state.layer_damage_rects.push(old_layer_damage);

        if record_undo {
            editor_state.push_undo();
        }
        apply_to_annotation(&mut editor_state.annotations[idx]);
        rebuild_annotation(editor_state, idx);
    }

    editor_state.settings_panel.dirty = true;
    let monitor_idx = editor_state.settings_panel.monitor_idx;
    if let Some(rect) = editor_state.settings_panel.rect() {
        editor_state
            .damage_rects
            .push(DamageZone::Local { monitor_idx, rect });
    }
    mark_dirty(dirty_mask, monitor_idx);

    apply_damage_rects(editor_state, dirty_mask);
}

/// Marks color popover for redraw and damages area it covers.
pub fn damage_color_popover(editor_state: &mut EditorState, dirty_mask: &mut u32) {
    editor_state.color_popover.dirty = true;
    let monitor_idx = editor_state.color_popover.monitor_idx;
    if let Some(rect) = editor_state.color_popover.rect() {
        emit_panel_damage(
            rect,
            monitor_idx,
            &mut editor_state.damage_rects,
            dirty_mask,
        );
    }
}
