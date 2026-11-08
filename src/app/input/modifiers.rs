//! 【截图输入】【修饰约束】按工具更新 Shift 和 Ctrl 的当前手势

use super::*;

/// 【界面】【修饰键更新】同步 editor_state 的 ctrl、shift，按工具约束更新未提交绘图；dirty_mask 返回重绘显示器
pub(crate) fn handle_canvas_modifiers(
    editor_state: &mut EditorState,
    ctrl: bool,
    shift: bool,
    dirty_mask: &mut u32,
) {
    let ctrl_changed = ctrl != editor_state.input.ctrl;
    let shift_changed = shift != editor_state.input.shift;
    let shift_pressed = shift && !editor_state.input.shift;
    editor_state.input.ctrl = ctrl;
    editor_state.input.shift = shift;
    if crate::app::annotation_collection::input::modifiers(
        editor_state,
        ctrl_changed || shift_changed,
        dirty_mask,
    ) {
        return;
    }
    if shift_changed
        && editor_state.selected_tool == Tool::Selection
        && editor_state.input.mouse_down
        && editor_state.input.drag_start.is_some()
    {
        dispatch_move(
            Tool::Selection,
            editor_state,
            editor_state.input.pointer.global,
            dirty_mask,
        );
        apply_damage_rects(editor_state, dirty_mask);
    }
    let constrained = matches!(
        editor_state.selected_tool,
        Tool::Ruler | Tool::Magnifier | Tool::Line | Tool::Arrow | Tool::Rectangle | Tool::Circle
    ) || (editor_state.selected_tool == Tool::Pick
        && editor_state.ann_drag.as_ref().is_some_and(|drag| {
            matches!(
                drag.orig.shape,
                crate::types::AnnotationShape::Redact { .. }
                    | crate::types::AnnotationShape::Ruler { .. }
                    | crate::types::AnnotationShape::Magnifier { .. }
                    | crate::types::AnnotationShape::Number { .. }
                    | crate::types::AnnotationShape::Line { .. }
                    | crate::types::AnnotationShape::Arrow { .. }
            )
        }));
    if constrained
        && (ctrl_changed || shift_changed)
        && (editor_state.pending.is_some() || editor_state.ann_drag.is_some())
    {
        let global = editor_state.input.pointer.global;
        dispatch_move(editor_state.selected_tool, editor_state, global, dirty_mask);
        apply_damage_rects(editor_state, dirty_mask);
        return;
    }
    if !shift {
        editor_state.input.straight = false;
    }
    if (shift_pressed || (ctrl_changed && editor_state.selected_tool == Tool::Marker))
        && editor_state.pending.is_some()
        && editor_state.ann_drag.is_none()
    {
        let global = editor_state.input.pointer.global;
        dispatch_move(editor_state.selected_tool, editor_state, global, dirty_mask);
        apply_damage_rects(editor_state, dirty_mask);
    }
}
