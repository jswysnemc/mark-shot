//! 【截图输入】【指针轨迹】坐标映射与画布移动分派

use super::*;

/// 【界面】【指针移动】转换 x、y 所属 monitor_idx 的坐标并更新 editor_state；dirty_mask 返回重绘显示器
pub(crate) fn handle_canvas_pointer_move(
    editor_state: &mut EditorState,
    monitor_idx: usize,
    x: f64,
    y: f64,
    dirty_mask: &mut u32,
) {
    let global = (
        editor_state.placements[monitor_idx].position.0 as f64 + x,
        editor_state.placements[monitor_idx].position.1 as f64 + y,
    );
    let (current_monitor_idx, local_x, local_y) =
        global_point_to_local(&editor_state.placements, global, monitor_idx, (x, y));
    update_pointer(
        editor_state,
        current_monitor_idx,
        (local_x, local_y),
        global,
    );
    editor_state.selection_size.clicks.moved(global);
    if selection_handles::motion(editor_state, dirty_mask) {
        apply_damage_rects(editor_state, dirty_mask);
        return;
    }
    if crate::app::selection_translation::input::pointer(editor_state, dirty_mask) {
        apply_damage_rects(editor_state, dirty_mask);
        return;
    }
    if crate::app::external_actions::pointer(editor_state, dirty_mask) {
        return;
    }
    if text_context_menu::pointer(editor_state, dirty_mask) {
        apply_damage_rects(editor_state, dirty_mask);
        return;
    }
    if crate::app::selection_size::input::pointer(editor_state, dirty_mask) {
        apply_damage_rects(editor_state, dirty_mask);
        return;
    }
    update_magnifier(editor_state, dirty_mask);
    if crate::features::color_sample::inspector::pointer(editor_state, dirty_mask) {
        apply_damage_rects(editor_state, dirty_mask);
        return;
    }
    if editor_state.toolbar_drag.is_some() {
        pointer_policy::move_toolbar(editor_state, dirty_mask);
        apply_damage_rects(editor_state, dirty_mask);
        return;
    }
    if text_properties::pointer_move(editor_state, dirty_mask) || editor_state.font_panel.open {
        font_popup::pointer_move(editor_state, dirty_mask);
        apply_damage_rects(editor_state, dirty_mask);
        return;
    }
    if crate::tools::text::session_input::pointer_move(
        editor_state,
        editor_state.canvas_point(),
        dirty_mask,
    ) {
        apply_damage_rects(editor_state, dirty_mask);
        return;
    }
    if crate::app::region_detection::input::pointer(editor_state, dirty_mask) {
        apply_damage_rects(editor_state, dirty_mask);
        return;
    }
    dispatch_move(editor_state.selected_tool, editor_state, global, dirty_mask);
    update_toolbar(editor_state, dirty_mask);
    if editor_state.settings_panel.visible {
        update_settings_panel(editor_state, dirty_mask);
    }
    if editor_state.color_popover.open {
        update_color_popover(editor_state, dirty_mask);
        handle_color_popover_drag(editor_state, dirty_mask);
    }
    if editor_state.model_popover.open {
        update_model_popover(editor_state, dirty_mask);
    }
    if editor_state.marker_popover.is_visible() {
        marker::pointer_move(editor_state, dirty_mask);
    }
    if editor_state.ruler_popover.is_visible() {
        ruler::pointer_move(editor_state, dirty_mask);
    }
    if editor_state.number_popover.is_visible() {
        number::pointer_move(editor_state, dirty_mask);
    }
    if editor_state.arrow_popover.is_visible() || editor_state.line_popover.is_visible() {
        line::pointer_move(editor_state, dirty_mask);
    }
    if editor_state.filter_popover.is_visible() {
        filter::pointer_move(editor_state, dirty_mask);
    }
    apply_damage_rects(editor_state, dirty_mask);
}
