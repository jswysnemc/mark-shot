//! 【标注】【文字工具】管理文字创建、焦点和拖动

use super::*;

impl ToolBehavior for TextTool {
    /// 【文字事务】【工具点击】按 state 的 button、pressed 开始新建或重新编辑，dirty 返回范围；文字区域不启动标注拖动
    fn on_button(
        &self,
        state: &mut EditorState,
        button: MouseButton,
        pressed: bool,
        dirty: &mut u32,
    ) {
        if !matches!(button, MouseButton::Left) || !pressed {
            return;
        }
        if state.text_session.is_some() {
            return;
        }
        if let Some(id) = state
            .annotations
            .iter()
            .rev()
            .find(|annotation| {
                matches!(annotation.shape, AnnotationShape::Text { .. })
                    && annotation.initial_hit_test(state.canvas_point())
            })
            .map(|annotation| annotation.id)
        {
            crate::editor::text_session::begin_existing(state, id, dirty);
            return;
        }
        let Some(space) = crate::tools::drawing_space::DrawingSpace::from_editor(state) else {
            return;
        };
        let Some(point) = space.start_point(state.canvas_point()) else {
            return;
        };
        if !state.selection.zone.is_some_and(|rect| {
            point.0 >= rect.left()
                && point.0 <= rect.right()
                && point.1 >= rect.top()
                && point.1 <= rect.bottom()
        }) {
            return;
        }
        crate::editor::text_session::begin_new(state, point, 1.0 / space.pixel_scale, dirty);
    }

    /// 【文字事务】【指针移动】按 global 更新 state 的已有标注手势，_dirty_mask 由上层应用损坏区域，无返回值
    fn on_move(&self, state: &mut EditorState, global: (f64, f64), _dirty_mask: &mut u32) {
        apply_annotation_drag(state, global);
    }

    /// 【文字事务】【字符输入】把 ch 写入 state 的预览，_dirty_mask 返回重绘范围；旧编辑器状态保持输入兼容，无返回值
    fn on_text(&self, state: &mut EditorState, ch: char, _dirty_mask: &mut u32) {
        if state.text_session.is_some() {
            super::session_input::text(state, &ch.to_string(), _dirty_mask);
            return;
        }
        if state.ann_drag.is_some() {
            return;
        }

        let Some(edit) = state.text.editing.as_ref() else {
            return;
        };
        let id = edit.annotation_id;

        if let Some(ann) = state.annotations.iter().find(|a| a.id == id) {
            state.layer_damage_rects.push(ann.damage_bbox(false));
            state
                .damage_rects
                .push(DamageZone::Global(ann.damage_bbox(true)));
        }

        if let Some(editor) = state.text.editors.get_mut(&id) {
            editor.action(&mut state.text.font_system, Action::Insert(ch));
            sync_content_from_editor(
                id,
                editor,
                &mut state.annotations,
                &mut state.text.font_system,
            );
        }

        if let Some(ann) = state.annotations.iter().find(|a| a.id == id) {
            state.layer_damage_rects.push(ann.damage_bbox(false));
            state
                .damage_rects
                .push(DamageZone::Global(ann.damage_bbox(true)));
        }
    }

    /// 【文字事务】【特殊按键】处理 state 的 key，_dirty_mask 由上层应用范围；事务编辑器先由统一输入入口处理，无返回值
    fn on_key(&self, state: &mut EditorState, key: SpecialKey, _dirty_mask: &mut u32) {
        if state.ann_drag.is_some() {
            return;
        }

        let Some(edit) = state.text.editing.as_ref() else {
            return;
        };
        let id = edit.annotation_id;
        let ctrl = state.input.ctrl;
        let shift = state.input.shift;

        if let Some(ann) = state.annotations.iter().find(|a| a.id == id) {
            state.layer_damage_rects.push(ann.damage_bbox(false));
            state
                .damage_rects
                .push(DamageZone::Global(ann.damage_bbox(true)));
        }

        if let Some(editor) = state.text.editors.get_mut(&id) {
            let text_changed =
                apply_key_to_editor(editor, &mut state.text.font_system, key, ctrl, shift);
            if text_changed {
                sync_content_from_editor(
                    id,
                    editor,
                    &mut state.annotations,
                    &mut state.text.font_system,
                );
            }
        }

        if let Some(ann) = state.annotations.iter().find(|a| a.id == id) {
            state.layer_damage_rects.push(ann.damage_bbox(false));
            state
                .damage_rects
                .push(DamageZone::Global(ann.damage_bbox(true)));
        }
    }

    /// 【文字事务】【工具退出】提交 state 的预览并结束字符选择，_dirty_mask 返回范围，无返回值
    fn on_deactivate(&self, state: &mut EditorState, _dirty_mask: &mut u32) {
        crate::editor::text_session::commit(state, _dirty_mask);
        if let Some(edit) = state.text.editing.take() {
            if let Some(ann) = state
                .annotations
                .iter()
                .find(|a| a.id == edit.annotation_id)
            {
                state.layer_damage_rects.push(ann.damage_bbox(false));
                state
                    .damage_rects
                    .push(DamageZone::Global(ann.damage_bbox(true)));
            }
            if let Some(editor) = state.text.editors.get_mut(&edit.annotation_id) {
                editor.set_selection(Selection::None);
            }
        }
        if let Some(old_idx) = state.take_selected_annotation()
            && let Some(old_ann) = state.annotations.get(old_idx)
        {
            state.layer_damage_rects.push(old_ann.damage_bbox(false));
            state
                .damage_rects
                .push(DamageZone::Global(old_ann.damage_bbox(true)));
            if let Some(editor) = state.text.editors.get_mut(&old_ann.id) {
                editor.set_selection(Selection::None);
            }
        }
        state.text.editing = None;
        state.select_annotation_index(None);
        state.annotations_dirty = true;
    }

    /// 【文字事务】【工具光标】按 state 的手势返回文字或控制点光标，不改变编辑器
    fn cursor(&self, state: &EditorState) -> CursorIcon {
        if let Some(drag) = &state.ann_drag {
            return cursor_for_handle(drag.handle, true).unwrap_or(CursorIcon::Text);
        }
        if let Some(ann) = state
            .selected_annotation_index()
            .and_then(|idx| state.annotations.get(idx))
            && matches!(ann.shape, AnnotationShape::Text { .. })
            && let Some(icon) =
                cursor_for_handle(state.canvas_handle(ann, state.canvas_point()), false)
        {
            return icon;
        }
        CursorIcon::Text
    }
}
