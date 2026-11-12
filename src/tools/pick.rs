use crate::editor::annotation_drag::{
    apply_annotation_drag, begin_drag_for_annotation, commit_drag_if_changed,
};
use crate::editor::dirty::damage_annotation;
use crate::editor::{DamageZone, EditorState};
use crate::tools::ToolBehavior;
use crate::types::{AnnDragState, CursorIcon, MouseButton, SelectionHandle};
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
        if crate::app::annotation_collection::input::button(state, button, pressed, _dirty_mask) {
            return;
        }

        if pressed {
            let mut selected_annotation = state.selected_annotation_index().filter(|index| {
                state.annotations.get(*index).is_some_and(|annotation| {
                    state.canvas_handle(annotation, state.canvas_point()) != SelectionHandle::None
                })
            });
            // 1. 【标注选择】【鼠标按钮】先检查当前选择的全部控制点，粗笔迹的缩放点也可能位于正文命中范围之外
            if selected_annotation.is_none() {
                for (i, ann) in state.annotations.iter().enumerate().rev() {
                    if state.canvas_body_hit(ann, state.canvas_point()) {
                        selected_annotation = Some(i);
                        break;
                    }
                }
            }

            let selection_changed = state.selected_annotation_index() != selected_annotation;
            if selection_changed {
                state.line_session.clear();
            }

            // nothing selected, nothing was selected > nothing to do
            if selected_annotation.is_none() && state.selected_annotation_index().is_none() {
                state.input.clicks.clear();
                state.begin_collection_marquee();
                return;
            }

            // select empty space > deselect
            if selected_annotation.is_none() {
                state.input.clicks.clear();
                if let Some(old_idx) = state.selected_annotation_index()
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
                state.select_annotation_index(None);
                state.ann_drag = None;
                state.text.editing = None;
                state.annotations_dirty = true;
                state.begin_collection_marquee();
                return;
            }

            // select a different annotation -> switch selection, no undo commit
            if selection_changed {
                if let Some(old_idx) = state.selected_annotation_index()
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
                state.select_annotation_index(selected_annotation);
                let idx = selected_annotation.unwrap();
                if crate::app::text_reedit::begin(state, idx, _dirty_mask)
                    || register_line_click(state, idx, _dirty_mask)
                {
                    return;
                }
                let ann = &state.annotations[idx];
                state
                    .damage_rects
                    .push(DamageZone::Global(ann.damage_bbox(true)));
                state.text.editing = None;

                // 2. 【标注选择】【鼠标按钮】首次正文命中后执行完整控制点判断，与原版按下入口一致
                let handle = state.canvas_handle(ann, state.canvas_point());
                let handle = if handle == SelectionHandle::None {
                    SelectionHandle::Move
                } else {
                    handle
                };
                if ann.shape.line().is_some() {
                    state.line_session.selected_point = match handle {
                        SelectionHandle::LinePoint(point) => Some((ann.id, point)),
                        _ => None,
                    };
                }
                state.ann_drag = Some(AnnDragState {
                    handle,
                    start_global: state.canvas_point(),
                    prev_global: state.canvas_point(),
                    orig: ann.clone(),
                    orig_index: idx,
                });
                return;
            }

            // click the same annotation > handle hit test (Move or resize), push undo before drag
            if let Some(idx) = state.selected_annotation_index() {
                if crate::app::text_reedit::begin(state, idx, _dirty_mask)
                    || register_line_click(state, idx, _dirty_mask)
                {
                    return;
                }
                begin_drag_for_annotation(state, idx);
            }
        } else {
            commit_drag_if_changed(state);
        }
    }

    /// 【画布导航】【选择移动】按global稳定画布位置更新state单项或群组，_dirty_mask返回重绘范围，无返回值
    fn on_move(&self, state: &mut EditorState, global: (f64, f64), _dirty_mask: &mut u32) {
        if crate::app::annotation_collection::input::pointer(state, global, _dirty_mask) {
            return;
        }
        apply_annotation_drag(state, global);
    }

    /// 【标注选择】【工具退出】完成 state 的已有标注拖动并清除选择；_dirty_mask 由调用方应用损坏区域，无返回值
    fn on_deactivate(&self, state: &mut EditorState, _dirty_mask: &mut u32) {
        state.line_session.clear();
        state.end_collection_interaction();
        commit_drag_if_changed(state);
        state.select_annotation_ids([]);
        state.ann_drag = None;
    }

    /// 【标注选择】【指针形状】按 state 当前删除按钮、拖动及控制点返回光标，不修改选择或历史
    fn cursor(&self, state: &EditorState) -> CursorIcon {
        if crate::app::annotation_selection::hovered(state) {
            return CursorIcon::Pointer;
        }
        if state.collection.marquee.is_some() {
            return CursorIcon::Crosshair;
        }
        if let Some(drag) = &state.collection.drag {
            return cursor_for_handle(drag.handle, true).unwrap_or(CursorIcon::Default);
        }
        if state.selected_annotation_ids().len() > 1 {
            return state
                .collection_bounds()
                .and_then(|bounds| {
                    cursor_for_handle(
                        state.canvas_collection_handle(bounds, state.canvas_point()),
                        false,
                    )
                })
                .unwrap_or(CursorIcon::Default);
        }
        if let Some(drag) = &state.ann_drag {
            return cursor_for_handle(drag.handle, true).unwrap_or(CursorIcon::Default);
        }
        let Some(ann) = state
            .selected_annotation_index()
            .and_then(|idx| state.annotations.get(idx))
        else {
            return CursorIcon::Default;
        };
        let handle = state.canvas_handle(ann, state.canvas_point());
        cursor_for_handle(handle, false).unwrap_or(CursorIcon::Default)
    }
}

/// 【标注选择】【线条双击】登记 state 中 index 线条的点击并尝试插入控制点；dirty_mask 返回重绘，返回是否完成插入
fn register_line_click(state: &mut EditorState, index: usize, dirty_mask: &mut u32) -> bool {
    let annotation = &state.annotations[index];
    if annotation.shape.line().is_none() {
        return false;
    }
    let id = annotation.id;
    let point =
        annotation.to_unrotated((state.canvas_point().0 as f32, state.canvas_point().1 as f32));
    state
        .line_session
        .register_click(id, point, std::time::Instant::now())
        && crate::editor::line_edits::insert_control(state, index, point, dirty_mask)
}

/// 【标注选择】【键盘删除】按 state 先删除线条已选控制点，再删除标注；dirty_mask 返回实际重绘，无返回值
pub fn delete_selected(state: &mut EditorState, dirty_mask: &mut u32) {
    if crate::editor::line_edits::remove_selected_control(state, dirty_mask) {
        return;
    }
    delete_annotation(state, dirty_mask);
}

/// 【标注选择】【整项删除】删除 state 当前完整标注并保留删除前选择身份，dirty_mask 返回重绘范围，无返回值
pub(crate) fn delete_annotation(state: &mut EditorState, dirty_mask: &mut u32) {
    state.line_session.clear();
    if state.selected_annotation_ids().len() > 1 {
        state.delete_collection(dirty_mask);
        return;
    }
    if let Some(idx) = state.selected_annotation_index()
        && idx < state.annotations.len()
    {
        state.push_undo();
        state.select_annotation_index(None);
        let ann = state.annotations.remove(idx);
        state.filters.remove(ann.id);
        damage_annotation(&mut state.damage_rects, &mut state.layer_damage_rects, &ann);
        state.ann_drag = None;
        state.annotations_dirty = true;
        *dirty_mask = u32::MAX;
    }
}
