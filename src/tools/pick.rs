use crate::editor::annotation_drag::{
    apply_annotation_drag, begin_drag_for_annotation, commit_drag_if_changed,
    handle_hit_test_for_annotation,
};
use crate::editor::dirty::damage_annotation;
use crate::editor::{DamageZone, EditorState};
use crate::tools::ToolBehavior;
use crate::types::{AnnDragState, AnnotationShape, CursorIcon, MouseButton, SelectionHandle};
use crate::utils::cursor_for_handle;
use cosmic_text::Edit;

pub struct PickTool;

impl ToolBehavior for PickTool {
    /// 【标注选择】【鼠标按钮】处理 state 中 button 的 pressed 状态；优先命中已选独立控制点，_dirty_mask 由调用方应用损坏区域
    fn on_button(
        &self,
        state: &mut EditorState,
        button: MouseButton,
        pressed: bool,
        _dirty_mask: &mut u32,
    ) {
        if !matches!(button, MouseButton::Left) {
            return;
        }

        if pressed {
            let mut selected_annotation = state.selected_annotation.filter(|index| {
                state.annotations.get(*index).is_some_and(|annotation| {
                    matches!(
                        annotation.shape,
                        AnnotationShape::Redact { .. }
                            | AnnotationShape::Ruler { .. }
                            | AnnotationShape::Magnifier { .. }
                            | AnnotationShape::Number { .. }
                            | AnnotationShape::Line { .. }
                            | AnnotationShape::Arrow { .. }
                    ) && handle_hit_test_for_annotation(annotation, state.input.pointer.global)
                        != SelectionHandle::None
                })
            });
            // 1. 【标注选择】【鼠标按钮】外框与旋转把手位于正文之外，先检查已选尺子、放大镜或序号，再查普通标注
            if selected_annotation.is_none() {
                for (i, ann) in state.annotations.iter().enumerate().rev() {
                    if ann.initial_hit_test(state.input.pointer.global) {
                        selected_annotation = Some(i);
                        break;
                    }
                }
            }

            if state.selected_annotation != selected_annotation {
                state.line_session.clear();
            }
            if let Some(index) = selected_annotation {
                let annotation = &state.annotations[index];
                if annotation.shape.line().is_some() {
                    let id = annotation.id;
                    let point = (
                        state.input.pointer.global.0 as f32,
                        state.input.pointer.global.1 as f32,
                    );
                    if state
                        .line_session
                        .register_click(id, point, std::time::Instant::now())
                        && crate::editor::line_edits::insert_control(
                            state,
                            index,
                            point,
                            _dirty_mask,
                        )
                    {
                        return;
                    }
                }
            }

            // nothing selected, nothing was selected > nothing to do
            if selected_annotation.is_none() && state.selected_annotation.is_none() {
                return;
            }

            // select empty space > deselect
            if selected_annotation.is_none() {
                if let Some(old_idx) = state.selected_annotation
                    && let Some(old_ann) = state.annotations.get(old_idx)
                {
                    damage_annotation(
                        &mut state.damage_rects,
                        &mut state.layer_damage_rects,
                        old_ann,
                    );
                    if let Some(editor) = state.text.editors.get_mut(&old_ann.id) {
                        editor.set_selection(cosmic_text::Selection::None);
                    }
                }
                state.selected_annotation = None;
                state.ann_drag = None;
                state.text.editing = None;
                state.annotations_dirty = true;
                return;
            }

            // select a different annotation -> switch selection, no undo commit
            if state.selected_annotation != selected_annotation {
                if let Some(old_idx) = state.selected_annotation
                    && let Some(old_ann) = state.annotations.get(old_idx)
                {
                    damage_annotation(
                        &mut state.damage_rects,
                        &mut state.layer_damage_rects,
                        old_ann,
                    );
                    if let Some(editor) = state.text.editors.get_mut(&old_ann.id) {
                        editor.set_selection(cosmic_text::Selection::None);
                    }
                }
                state.selected_annotation = selected_annotation;
                let idx = selected_annotation.unwrap();
                let ann = &state.annotations[idx];
                state
                    .damage_rects
                    .push(DamageZone::Global(ann.damage_bbox(true)));
                state.text.editing = None;

                // 2. 【标注选择】【鼠标按钮】首次选中双框或序号时按实际定点拖动，不使用尚未显示的缩放控制点
                let handle = if let Some(line) = ann.shape.line() {
                    let handle = crate::features::line::drag::hit_test(
                        line,
                        ann.bbox,
                        ann.stroke_width,
                        (
                            state.input.pointer.global.0 as f32,
                            state.input.pointer.global.1 as f32,
                        ),
                        false,
                    );
                    if handle == SelectionHandle::None {
                        SelectionHandle::Move
                    } else {
                        handle
                    }
                } else if let AnnotationShape::Magnifier { magnifier } = ann.shape {
                    crate::features::magnifier::drag::hit_test(
                        magnifier,
                        (
                            state.input.pointer.global.0 as f32,
                            state.input.pointer.global.1 as f32,
                        ),
                        false,
                    )
                } else if let AnnotationShape::Number { mark } = ann.shape {
                    crate::features::number::drag::hit_test(
                        mark,
                        ann.stroke_width,
                        (
                            state.input.pointer.global.0 as f32,
                            state.input.pointer.global.1 as f32,
                        ),
                        false,
                    )
                } else {
                    SelectionHandle::Move
                };
                if ann.shape.line().is_some() {
                    state.line_session.selected_point = match handle {
                        SelectionHandle::LinePoint(point) => Some((ann.id, point)),
                        _ => None,
                    };
                }
                state.ann_drag = Some(AnnDragState {
                    handle,
                    start_global: state.input.pointer.global,
                    prev_global: state.input.pointer.global,
                    orig: ann.clone(),
                    orig_index: idx,
                });
                return;
            }

            // click the same annotation > handle hit test (Move or resize), push undo before drag
            if let Some(idx) = state.selected_annotation {
                begin_drag_for_annotation(state, idx);
            }
        } else {
            commit_drag_if_changed(state);
        }
    }

    fn on_move(&self, state: &mut EditorState, global: (f64, f64), _dirty_mask: &mut u32) {
        apply_annotation_drag(state, global);
    }

    /// 【标注选择】【工具退出】完成 state 的已有标注拖动并清除选择；_dirty_mask 由调用方应用损坏区域，无返回值
    fn on_deactivate(&self, state: &mut EditorState, _dirty_mask: &mut u32) {
        state.line_session.clear();
        commit_drag_if_changed(state);
        if let Some(idx) = state.selected_annotation {
            state
                .damage_rects
                .push(DamageZone::Global(state.annotations[idx].damage_bbox(true)));
            state.selected_annotation = None;
            state.ann_drag = None;
        }
    }

    fn cursor(&self, state: &EditorState) -> CursorIcon {
        if let Some(drag) = &state.ann_drag {
            return cursor_for_handle(drag.handle, true).unwrap_or(CursorIcon::Default);
        }
        let Some(ann) = state
            .selected_annotation
            .and_then(|idx| state.annotations.get(idx))
        else {
            return CursorIcon::Default;
        };
        let handle = handle_hit_test_for_annotation(ann, state.input.pointer.global);
        cursor_for_handle(handle, false).unwrap_or(CursorIcon::Default)
    }
}

pub fn delete_selected(state: &mut EditorState, dirty_mask: &mut u32) {
    if crate::editor::line_edits::remove_selected_control(state, dirty_mask) {
        return;
    }
    state.line_session.clear();
    if let Some(idx) = state.selected_annotation.take()
        && idx < state.annotations.len()
    {
        state.push_undo();
        let ann = state.annotations.remove(idx);
        state.filters.remove(ann.id);
        damage_annotation(&mut state.damage_rects, &mut state.layer_damage_rects, &ann);
        state.ann_drag = None;
        state.annotations_dirty = true;
        *dirty_mask = u32::MAX;
    }
}
