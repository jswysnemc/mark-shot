mod modifiers;
mod pointer_move;
pub(crate) use modifiers::handle_canvas_modifiers;
pub(crate) use pointer_move::handle_canvas_pointer_move;
mod color_button;
mod keyboard;
pub(crate) mod pointer_policy;
mod recent_annotation;
pub(crate) mod selection_handles;
mod text_context_menu;
pub(crate) mod text_reedit;
pub(in crate::app) mod ui_hit;
pub(crate) mod wheel_preview;
pub use ui_hit::compute_cursor;

/// 【画布输入】【界面避让】按 state 返回当前指针是否处于无控件覆盖的画布，供实时采样使用
pub(crate) fn canvas_ui_free(state: &crate::editor::EditorState) -> bool {
    matches!(
        ui_hit::hit_test_ui(state, state.input.pointer.local),
        ui_hit::UiHit::None
    )
}
use ui_hit::{UiHit, hit_test_ui};
#[cfg(test)]
pub(in crate::app) mod tests;
pub(crate) use super::canvas_navigation::input::{
    handle_modifiers, handle_pointer_button, handle_pointer_move,
};
pub use keyboard::handle_key;
mod scroll;
use super::panels::color::{
    close_color_popover, commit_color_field_edit, handle_color_field_key_press,
    handle_color_field_text_input, handle_color_popover_click, handle_color_popover_drag,
    handle_color_popover_release, update_color_popover,
};
use super::panels::model::{close_model_popover, handle_model_popover_click, update_model_popover};
use super::panels::number;
use super::panels::ruler;
use super::panels::settings::{
    apply_stepper_arrow_step, apply_toggle_field, commit_stepper_text_edit,
    handle_settings_key_press, handle_settings_text_input, handle_stepper_scroll,
    run_settings_action, sync_stepper_edit_text, update_settings_panel,
};
use super::panels::text::{font_popup, properties as text_properties};
use super::panels::toolbar::update_toolbar;
use super::panels::{filter, line, marker};
use crate::editor::dirty::{apply_damage_rects, mark_all_dirty, mark_dirty};
use crate::editor::{DamageZone, EditorState};
use crate::interaction::ClickTarget;
use crate::keys::{self, Chord, Key};
use crate::theme::font;
use crate::tools::{
    Tool, dispatch_activate, dispatch_button, dispatch_key, dispatch_move, dispatch_text,
};
use crate::types::{MouseButton, PointerState, SpecialKey};
use crate::ui::magnifier::MagnifierState;
use crate::ui::panel::UiPanel;
use crate::ui::settings_panel::char_index_for_x;
use crate::ui::settings_panel::{ArrowHoldState, SettingsWidget, StepperArrow};
use crate::ui::text_field::CursorInit;
use crate::ui::toolbar::{ToolbarButton, ToolbarItem};
use crate::utils::{get_full_workspace_rect, global_point_to_local};
pub use scroll::handle_axis_scroll;
#[cfg(test)]
pub use scroll::handle_scroll;
use scroll::languages_button_hit;
use std::time::Instant;
/// 【界面】【鼠标按钮】将 button 的 pressed 状态分派到 editor_state 的弹层或工具；dirty_mask 返回重绘显示器
pub(crate) fn handle_canvas_pointer_button(
    editor_state: &mut EditorState,
    button: MouseButton,
    pressed: bool,
    dirty_mask: &mut u32,
) {
    if crate::features::color_sample::inspector::button(editor_state, button, pressed, dirty_mask) {
        apply_damage_rects(editor_state, dirty_mask);
        return;
    }
    if !pressed && selection_handles::button(editor_state, button, pressed, dirty_mask) {
        apply_damage_rects(editor_state, dirty_mask);
        return;
    }
    if super::annotation_selection::release(editor_state, button, pressed) {
        return;
    }
    // 1. 【选区尺寸】【平台点击】观测本次实际事件，其他界面消费按钮时也保留正确的平台点击序列
    editor_state.selection_size.clicks.button(
        button,
        pressed,
        editor_state.input.pointer.global,
        editor_state.input.pointer_timestamp,
        Instant::now(),
    );
    if super::selection_translation::input::button(editor_state, button, pressed, dirty_mask) {
        apply_damage_rects(editor_state, dirty_mask);
        return;
    }
    if pressed {
        editor_state.commit_text_size_wheel();
    }
    if super::external_actions::button(editor_state, button, pressed, dirty_mask) {
        return;
    }
    if text_context_menu::button(editor_state, button, pressed, dirty_mask) {
        apply_damage_rects(editor_state, dirty_mask);
        return;
    }
    if super::selection_size::input::button(editor_state, button, pressed, dirty_mask) {
        apply_damage_rects(editor_state, dirty_mask);
        return;
    }
    if pointer_policy::consume(editor_state, button, pressed, dirty_mask) {
        refresh_panels(editor_state, dirty_mask);
        apply_damage_rects(editor_state, dirty_mask);
        return;
    }
    if pressed {
        editor_state.input.right_clicks.clear();
    }
    if !pressed
        && (text_properties::finish(editor_state, true, dirty_mask)
            || font_popup::release(editor_state, dirty_mask))
    {
        refresh_panels(editor_state, dirty_mask);
        apply_damage_rects(editor_state, dirty_mask);
        return;
    }
    if !pressed && color_button::release(editor_state, dirty_mask) {
        apply_damage_rects(editor_state, dirty_mask);
        return;
    }
    if !pressed && editor_state.toolbar_drag.take().is_some() {
        return;
    }
    if !pressed && let Some((secondary, index, monitor)) = editor_state.input.toolbar_press.take() {
        let hit = hit_test_ui(editor_state, editor_state.input.pointer.local);
        let same = match hit {
            UiHit::ToolbarItem(current) => !secondary && current == index,
            UiHit::ActionToolbarItem(current) => secondary && current == index,
            _ => false,
        };
        if same && monitor == editor_state.input.pointer.monitor_idx {
            pointer_policy::toolbar_click(editor_state, secondary, index, dirty_mask);
        }
        apply_damage_rects(editor_state, dirty_mask);
        return;
    }
    if !pressed && !editor_state.input.left_canvas {
        editor_state.settings_panel.arrow_held = None;
        handle_color_popover_release(editor_state, dirty_mask);
        return;
    }
    if !pressed {
        editor_state.input.left_canvas = false;
    }
    editor_state.settings_panel.cancel_scroll();
    editor_state.color_popover.cancel_scroll();
    editor_state.marker_popover.cancel_scroll();
    editor_state.number_popover.cancel_scroll();
    editor_state.arrow_popover.cancel_scroll();
    editor_state.line_popover.cancel_scroll();
    editor_state.filter_popover.cancel_scroll();
    let is_left_click_pressed = matches!(button, MouseButton::Left) && pressed;

    if matches!(button, MouseButton::Left) && !pressed {
        editor_state.settings_panel.arrow_held = None;
        handle_color_popover_release(editor_state, dirty_mask);
    }
    if is_left_click_pressed && editor_state.settings_panel.is_editing() {
        commit_stepper_text_edit(editor_state, dirty_mask);
    }
    if is_left_click_pressed && editor_state.color_popover.fields.is_editing() {
        commit_color_field_edit(editor_state, dirty_mask);
    }

    // ── 1. priority ui hit test ──────────────────────────────────────────────

    if is_left_click_pressed {
        if editor_state.font_panel.open {
            if editor_state.input.pointer.monitor_idx == editor_state.font_panel.monitor_idx
                && editor_state
                    .font_panel
                    .hit_test(editor_state.input.pointer.local)
            {
                font_popup::press(editor_state, dirty_mask);
                apply_damage_rects(editor_state, dirty_mask);
                return;
            }
            if text_properties::hit(editor_state)
                == Some(crate::ui::text_properties::layout::Control::Font)
            {
                text_properties::press(editor_state, dirty_mask);
                return;
            }
            font_popup::close(editor_state, dirty_mask);
        }
        let mut ui_hit = hit_test_ui(editor_state, editor_state.input.pointer.local);

        if let UiHit::FilterPopoverOutside = ui_hit {
            let on_toggle = filter::toggle_button_hit(editor_state);
            filter::close(editor_state, dirty_mask);
            if on_toggle {
                apply_damage_rects(editor_state, dirty_mask);
                return;
            }
            ui_hit = hit_test_ui(editor_state, editor_state.input.pointer.local);
        }

        if let UiHit::LinePopoverOutside(kind) = ui_hit {
            let on_toggle = line::toggle_button_hit(editor_state, kind);
            line::close_one(editor_state, kind, dirty_mask);
            if on_toggle {
                apply_damage_rects(editor_state, dirty_mask);
                return;
            }
            ui_hit = hit_test_ui(editor_state, editor_state.input.pointer.local);
        }

        if let UiHit::NumberPopoverOutside = ui_hit {
            let on_toggle = number::toggle_button_hit(editor_state);
            number::close(editor_state, dirty_mask);
            if on_toggle {
                apply_damage_rects(editor_state, dirty_mask);
                return;
            }
            ui_hit = hit_test_ui(editor_state, editor_state.input.pointer.local);
        }

        if let UiHit::RulerPopoverOutside = ui_hit {
            let on_toggle = ruler::toggle_button_hit(editor_state);
            ruler::close(editor_state, dirty_mask);
            if on_toggle {
                apply_damage_rects(editor_state, dirty_mask);
                return;
            }
            ui_hit = hit_test_ui(editor_state, editor_state.input.pointer.local);
        }

        if let UiHit::MarkerPopoverOutside = ui_hit {
            let on_toggle = marker::toggle_button_hit(editor_state);
            marker::close(editor_state, dirty_mask);
            if on_toggle {
                apply_damage_rects(editor_state, dirty_mask);
                return;
            }
            ui_hit = hit_test_ui(editor_state, editor_state.input.pointer.local);
        }

        if let UiHit::ColorPopoverOutside = ui_hit {
            if color_button::press(editor_state) {
                return;
            }
            close_color_popover(editor_state, dirty_mask);
            ui_hit = hit_test_ui(editor_state, editor_state.input.pointer.local);
        }

        if let UiHit::ModelPopoverOutside = ui_hit {
            let on_toggle = languages_button_hit(editor_state);
            close_model_popover(editor_state, dirty_mask);
            if on_toggle {
                apply_damage_rects(editor_state, dirty_mask);
                return;
            }
            ui_hit = hit_test_ui(editor_state, editor_state.input.pointer.local);
        }

        match ui_hit {
            UiHit::FontPanelInside | UiHit::FontPanelOutside => {
                unreachable!("font panel is handled before generic controls")
            }
            UiHit::FilterPopoverInside => {
                filter::click(editor_state, dirty_mask);
                apply_damage_rects(editor_state, dirty_mask);
                return;
            }
            UiHit::FilterPopoverOutside => unreachable!("filter popover is closed at this point"),
            UiHit::LinePopoverInside(kind) => {
                line::click(editor_state, kind, dirty_mask);
                apply_damage_rects(editor_state, dirty_mask);
                return;
            }
            UiHit::LinePopoverOutside(_) => unreachable!("line popover is closed at this point"),
            UiHit::NumberPopoverInside => {
                number::click(editor_state, dirty_mask);
                apply_damage_rects(editor_state, dirty_mask);
                return;
            }
            UiHit::NumberPopoverOutside => unreachable!("number popover is closed at this point"),
            UiHit::RulerPopoverInside => {
                ruler::click(editor_state, dirty_mask);
                apply_damage_rects(editor_state, dirty_mask);
                return;
            }
            UiHit::RulerPopoverOutside => unreachable!("ruler popover is closed at this point"),
            UiHit::MarkerPopoverInside => {
                marker::click(editor_state, dirty_mask);
                apply_damage_rects(editor_state, dirty_mask);
                return;
            }
            UiHit::MarkerPopoverOutside => unreachable!("popover is closed at this point"),
            UiHit::ColorPopoverInside => {
                handle_color_popover_click(editor_state, dirty_mask);
                apply_damage_rects(editor_state, dirty_mask);
                return;
            }
            UiHit::ColorPopoverOutside => unreachable!("popover is closed at this point"),
            UiHit::ModelPopoverInside => {
                handle_model_popover_click(editor_state, dirty_mask);
                apply_damage_rects(editor_state, dirty_mask);
                return;
            }
            UiHit::ModelPopoverOutside => unreachable!("popover is closed at this point"),

            UiHit::ToolbarBackground => {
                pointer_policy::begin_toolbar_drag(editor_state, false);
                return;
            }
            UiHit::ActionToolbarBackground => {
                pointer_policy::begin_toolbar_drag(editor_state, true);
                return;
            }
            UiHit::ToolbarItem(index) => {
                editor_state.input.toolbar_press =
                    Some((false, index, editor_state.input.pointer.monitor_idx));
                return;
            }
            UiHit::ActionToolbarItem(index) => {
                editor_state.input.toolbar_press =
                    Some((true, index, editor_state.input.pointer.monitor_idx));
                return;
            }

            UiHit::SettingsBackground => {
                return;
            }

            UiHit::SettingsItem(widget_idx) => {
                if color_button::press(editor_state) {
                    return;
                }
                if text_properties::press(editor_state, dirty_mask) {
                    apply_damage_rects(editor_state, dirty_mask);
                    return;
                }
                let monitor_idx = editor_state.settings_panel.monitor_idx;
                editor_state.settings_panel.dirty = true;

                editor_state.settings_panel.selected = Some(widget_idx);

                match editor_state.settings_panel.widgets[widget_idx] {
                    SettingsWidget::Stepper { .. } => {
                        if let Some(arrow) = editor_state
                            .settings_panel
                            .stepper_arrow_hit(widget_idx, editor_state.input.pointer.local)
                        {
                            apply_stepper_arrow_step(editor_state, widget_idx, arrow, dirty_mask);
                            update_settings_panel(editor_state, dirty_mask);
                            editor_state.settings_panel.arrow_held = Some(ArrowHoldState {
                                widget_idx,
                                arrow,
                                started_at: Instant::now(),
                                last_step_at: Instant::now(),
                                repeat_count: 0,
                            });
                        } else {
                            let click_pos = (
                                editor_state.input.pointer.local.0 as f32,
                                editor_state.input.pointer.local.1 as f32,
                            );
                            let is_double_click = editor_state
                                .input
                                .clicks
                                .register(ClickTarget::SettingsWidget(widget_idx), click_pos);

                            let current_value = editor_state
                                .settings_panel
                                .fields
                                .value(widget_idx)
                                .cloned()
                                .unwrap_or_default();

                            let cursor_init = if is_double_click {
                                CursorInit::SelectAll
                            } else if let Some(text_x) =
                                editor_state.settings_panel.widget_text_x(widget_idx)
                            {
                                let click_x = editor_state.input.pointer.local.0 as f32 - text_x;
                                let idx = char_index_for_x(
                                    &current_value,
                                    click_x,
                                    font::label(),
                                    &mut editor_state.text.font_system,
                                );
                                CursorInit::At(idx)
                            } else {
                                CursorInit::End
                            };

                            editor_state.settings_panel.pre_edit_snapshot =
                                Some(editor_state.snapshot());
                            editor_state.settings_panel.begin_edit(
                                widget_idx,
                                current_value,
                                cursor_init,
                            );
                        }
                    }
                    SettingsWidget::Toggle { field, .. } => {
                        let new_val = editor_state.settings_panel.toggle(widget_idx);
                        apply_toggle_field(editor_state, field, new_val, dirty_mask);
                    }
                    SettingsWidget::ColorSwatch(_) => {
                        unreachable!("color button press is handled before settings dispatch")
                    }
                    SettingsWidget::Action { action, .. } => {
                        editor_state.settings_panel.selected = None;
                        run_settings_action(editor_state, action, dirty_mask);
                    }

                    SettingsWidget::Value { field } => {
                        editor_state.settings_panel.selected = None;
                        let color = super::panels::settings::current_color(editor_state);
                        crate::tools::eyedropper::copy_value(editor_state, field, color);
                    }
                    _ => {}
                }
                if editor_state.color_popover.open {
                    update_color_popover(editor_state, dirty_mask);
                }

                mark_dirty(dirty_mask, monitor_idx);

                if let Some(rect) = editor_state.settings_panel.rect() {
                    editor_state
                        .damage_rects
                        .push(DamageZone::Local { monitor_idx, rect });
                }

                apply_damage_rects(editor_state, dirty_mask);
                return;
            }

            UiHit::None => {}
        }
    }

    // ── 2. dispatch event to active tool ─────────────────────────────────────

    if is_left_click_pressed && selection_handles::button(editor_state, button, pressed, dirty_mask)
    {
        apply_damage_rects(editor_state, dirty_mask);
        return;
    }
    if is_left_click_pressed && super::annotation_selection::press(editor_state, dirty_mask) {
        return;
    }
    if is_left_click_pressed {
        editor_state.input.left_canvas = true;
        editor_state.settings_panel.dirty = true;
        editor_state.settings_panel.selected = None;
        let monitor_idx = editor_state.toolbar.monitor_idx;

        if let Some(rect) = editor_state.settings_panel.rect() {
            editor_state
                .damage_rects
                .push(DamageZone::Local { monitor_idx, rect });
        }
    }

    if is_left_click_pressed && editor_state.pick_once {
        crate::tools::eyedropper::pick(editor_state, dirty_mask);
        apply_damage_rects(editor_state, dirty_mask);
        return;
    }

    if crate::tools::text::session_input::button(editor_state, button, pressed, dirty_mask) {
        refresh_panels(editor_state, dirty_mask);
        apply_damage_rects(editor_state, dirty_mask);
        return;
    }

    super::region_detection::input::button(editor_state, button, pressed, dirty_mask);
    dispatch_button(
        editor_state.selected_tool,
        editor_state,
        button,
        pressed,
        dirty_mask,
    );

    super::region_detection::input::sync(editor_state, dirty_mask);

    if (matches!(button, MouseButton::Left) && !pressed) || matches!(button, MouseButton::Right) {
        update_toolbar(editor_state, dirty_mask);
        update_settings_panel(editor_state, dirty_mask);
    }

    apply_damage_rects(editor_state, dirty_mask);
}

/// 【界面】【工具切换】在 editor_state 中切换到 tool 并结束旧手势与弹层；dirty_mask 返回重绘显示器
pub fn select_tool(editor_state: &mut EditorState, tool: Tool, dirty_mask: &mut u32) {
    if editor_state.selected_tool != tool {
        editor_state.selection.creating = false;
    }
    super::selection_size::apply::commit(editor_state, dirty_mask);
    crate::editor::text_session::commit(editor_state, dirty_mask);
    let tb_button =
        editor_state.toolbar.items.iter().position(
            |item| matches!(item, ToolbarItem::Button(ToolbarButton::Tool(t)) if *t == tool),
        );
    if tool == Tool::Ruler && editor_state.selected_tool == Tool::Ruler {
        let mode = editor_state.tool_settings.ruler_mode.next();
        crate::editor::ruler_session::set_mode(editor_state, mode, true, dirty_mask);
        ruler::close(editor_state, dirty_mask);
        refresh_panels(editor_state, dirty_mask);
        apply_damage_rects(editor_state, dirty_mask);
        return;
    }
    if tool == Tool::Ruler && editor_state.selection.zone.is_none() {
        editor_state.ruler.entered_from_selection = true;
    } else if tool == Tool::Selection {
        editor_state.ruler.entered_from_selection = false;
    }
    editor_state.settings_panel.selected = None;
    if editor_state.color_popover.open {
        close_color_popover(editor_state, dirty_mask);
    }
    marker::close(editor_state, dirty_mask);
    ruler::close(editor_state, dirty_mask);
    number::close(editor_state, dirty_mask);
    line::close(editor_state, dirty_mask);
    filter::close(editor_state, dirty_mask);
    text_properties::finish(editor_state, false, dirty_mask);
    font_popup::close(editor_state, dirty_mask);

    crate::tools::eyedropper::end_pick_once(editor_state, dirty_mask);
    // ocr falls back to the monitor under the cursor,
    // so it doesn't want a forced full-workspace one

    // TODO:  better handling  of  cross  monitor ocr
    // i think there must be a visual indicator, that
    // ocr worked in ONE monitor, but  not  the other
    if editor_state.selection.zone.is_none() && tool != Tool::Selection && tool != Tool::Ocr {
        editor_state.selection.zone = get_full_workspace_rect(&editor_state.placements);
        mark_all_dirty(dirty_mask, editor_state.placements.len());
    }
    super::annotation_collection::input::deactivate(editor_state, tool, dirty_mask);
    editor_state.selected_tool = tool;
    editor_state.toolbar.selected = tb_button;
    editor_state.action_toolbar.dirty = true;
    editor_state.action_toolbar.selected =
        editor_state.action_toolbar.items.iter().position(
            |item| matches!(item, ToolbarItem::Button(ToolbarButton::Tool(t)) if *t == tool),
        );
    editor_state.toolbar.dirty = true;
    dispatch_activate(tool, editor_state, dirty_mask);

    refresh_panels(editor_state, dirty_mask);
    apply_damage_rects(editor_state, dirty_mask);
}

/// 【界面】【面板同步】根据 editor_state 的当前工具和选中标注刷新面板；dirty_mask 返回重绘显示器
pub fn refresh_panels(editor_state: &mut EditorState, dirty_mask: &mut u32) {
    update_toolbar(editor_state, dirty_mask);
    update_settings_panel(editor_state, dirty_mask);
    font_popup::update(editor_state, dirty_mask);
    if editor_state.color_popover.open {
        update_color_popover(editor_state, dirty_mask);
        handle_color_popover_drag(editor_state, dirty_mask);
    }
    if editor_state.model_popover.is_visible() {
        update_model_popover(editor_state, dirty_mask);
    }
    if editor_state.marker_popover.is_visible() {
        marker::update(editor_state, dirty_mask);
    }
    if editor_state.ruler_popover.is_visible() {
        ruler::update(editor_state, dirty_mask);
    }
    if editor_state.number_popover.is_visible() {
        number::update(editor_state, dirty_mask);
    }
    if editor_state.arrow_popover.is_visible() || editor_state.line_popover.is_visible() {
        line::update(editor_state, dirty_mask);
    }
    if editor_state.filter_popover.is_visible() {
        filter::update(editor_state, dirty_mask);
    }
    super::selection_size::panels::update(editor_state, dirty_mask);
    super::selection_translation::panels::update(editor_state, dirty_mask);
}

fn update_pointer(
    editor_state: &mut EditorState,
    monitor_idx: usize,
    local: (f64, f64),
    global: (f64, f64),
) {
    editor_state.input.pointer = PointerState::new(monitor_idx, local, global);
}

fn update_magnifier(editor_state: &mut EditorState, dirty_mask: &mut u32) {
    let monitor_idx = editor_state.input.pointer.monitor_idx;
    let local = editor_state.input.pointer.local;

    if let Some(mag) = editor_state.magnifier.current.as_ref()
        && mag.monitor_idx != monitor_idx
        && editor_state.magnifier_shown()
    {
        mark_dirty(dirty_mask, mag.monitor_idx);
    }

    editor_state.magnifier.current = Some(MagnifierState {
        monitor_idx,
        pos: local,
    });
    if editor_state.magnifier_shown() {
        mark_dirty(dirty_mask, monitor_idx);
    }
}
