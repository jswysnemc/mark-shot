// kind of a hotkey manager
// transfers key actions to it respective function, depends of the config
// handles some special cases
// and overall is context aware, and does some safety checks
use tiny_skia::Rect;

use crate::editor::dirty::{apply_damage_rects, mark_all_dirty, mark_dirty};
use crate::editor::{DamageZone, EditorState};
use crate::keys::{Action, Dir};
use crate::tools::Tool;
use crate::types::{Finish, Mode};
use crate::ui::magnifier::magnifier_rect;
use crate::ui::settings_panel::{SettingsWidget, StepperArrow, ValueField};
use crate::utils::get_full_workspace_rect;

use super::input::{refresh_panels, select_tool};
use super::panels::color::close_color_popover;
use super::panels::model::close_model_popover;
use super::panels::settings::{
    apply_stepper_arrow_step, sync_stepper_edit_text, update_settings_panel,
};

/// 【界面】【动作分派】在 editor_state 中执行 action 并处理工具上下文；dirty_mask 返回重绘显示器，无返回值
pub fn run(editor_state: &mut EditorState, action: Action, dirty_mask: &mut u32) {
    if editor_state.selection_translation.busy() {
        if action == Action::Cancel {
            super::selection_translation::tasks::cancel(editor_state, dirty_mask);
        }
        return;
    }
    if editor_state.upload.busy() {
        if action == Action::Cancel {
            super::upload::cancel(editor_state, dirty_mask);
        }
        return;
    }
    if matches!(
        action,
        Action::Accept
            | Action::Finish(_)
            | Action::SaveAs
            | Action::OpenWith
            | Action::Extensions
            | Action::Upload
    ) {
        crate::editor::text_session::commit(editor_state, dirty_mask);
    }
    if editor_state.mode == Mode::Ocr
        && !matches!(
            action,
            Action::Cancel | Action::ToggleMagnifier | Action::ToggleTheme | Action::History
        )
    {
        return;
    }
    editor_state.settings_panel.cancel_scroll();
    editor_state.color_popover.cancel_scroll();

    // when dragging doesn't allow to change tool and etc
    let busy = editor_state.tool_active
        || editor_state.input.mouse_down
        || editor_state.color_popover.sv_square.dragging
        || editor_state.color_popover.hue_dragging
        || editor_state.color_popover.alpha_dragging;
    match action {
        Action::Translate => super::selection_translation::toggle(editor_state, dirty_mask),
        Action::Upload => super::upload::request(editor_state, dirty_mask),
        Action::OpenWith => super::external_actions::toggle(
            editor_state,
            crate::ui::external_panel::Kind::OpenWith,
            dirty_mask,
        ),
        Action::Extensions => super::external_actions::toggle(
            editor_state,
            crate::ui::external_panel::Kind::Extensions,
            dirty_mask,
        ),
        Action::Accept => {
            if !tool_copy(editor_state, dirty_mask) && !editor_state.to.is_empty() {
                editor_state.finish = Some(editor_state.to);
            }
        }
        Action::Finish(Finish::Copy) => {
            if !tool_copy(editor_state, dirty_mask) {
                editor_state.finish = Some(Finish::Copy.into());
            }
        }
        Action::Finish(finish) => editor_state.finish = Some(finish.into()),
        Action::SaveAs
            if !busy
                && editor_state.pending.is_none()
                && editor_state.ann_drag.is_none()
                && editor_state
                    .selection
                    .zone
                    .is_some_and(|region| region.width() >= 1.0 && region.height() >= 1.0) =>
        {
            editor_state.save_as_requested = true;
        }
        Action::SaveAs => {}
        Action::Cancel => {
            if editor_state.ann_drag.as_ref().is_some_and(|drag| {
                matches!(
                    drag.orig.shape,
                    crate::types::AnnotationShape::Redact { .. }
                        | crate::types::AnnotationShape::Magnifier { .. }
                        | crate::types::AnnotationShape::Number { .. }
                        | crate::types::AnnotationShape::Line { .. }
                        | crate::types::AnnotationShape::Arrow { .. }
                )
            }) {
                crate::editor::EditorState::undo(editor_state, dirty_mask);
                refresh_panels(editor_state, dirty_mask);
                return;
            }
            if editor_state.selected_tool != Tool::Ruler
                && editor_state.selected_tool.is_drawing()
                && editor_state.pending.is_some()
                && editor_state.ann_drag.is_none()
            {
                // 1. 【标注】【取消草稿】先取消未提交图形，不关闭已有标注所在的截图会话
                crate::editor::drawing_gesture::cancel(editor_state, dirty_mask);
                refresh_panels(editor_state, dirty_mask);
                return;
            }
            use crate::editor::ruler_session::CancelResult;
            match crate::editor::ruler_session::cancel_or_leave(editor_state, dirty_mask) {
                CancelResult::Unhandled => editor_state.cancel = true,
                CancelResult::Canceled => {}
                CancelResult::ReturnToSelection => {
                    select_tool(editor_state, Tool::Selection, dirty_mask);
                }
            }
        }
        Action::Undo => {
            editor_state.undo(dirty_mask);
            super::refresh_panels_after_history(editor_state, dirty_mask);
        }
        Action::Redo => {
            editor_state.redo(dirty_mask);
            super::refresh_panels_after_history(editor_state, dirty_mask);
        }
        Action::Delete => {
            if matches!(
                editor_state.selected_tool,
                Tool::Pick | Tool::Ruler | Tool::Magnifier
            ) {
                crate::tools::pick::delete_selected(editor_state, dirty_mask);
            }
        }
        Action::SelectAll => select_all(editor_state, dirty_mask),
        Action::ToggleUi | Action::Tool(_) if editor_state.region() => {}
        Action::ToggleUi => toggle_ui(editor_state, dirty_mask),
        Action::ToggleMagnifier => toggle_magnifier(editor_state, dirty_mask),
        Action::ToggleTheme => super::theme_switch::toggle(editor_state, dirty_mask),
        Action::ScrollCapture => begin_scroll_capture(editor_state, dirty_mask),
        Action::Settings => editor_state.preferences_requested = true,
        Action::ClearAnnotations | Action::ToggleScope | Action::ToggleToolbarLayout => {
            use crate::ui::toolbar::ToolbarButton;
            let button = match action {
                Action::ClearAnnotations => ToolbarButton::Clear,
                Action::ToggleScope => ToolbarButton::Scope,
                _ => ToolbarButton::Layout,
            };
            super::input::pointer_policy::command(editor_state, button, dirty_mask);
        }
        // 1. 【截图历史】【手势保护】旧画笔与选择拖动未设置忙碌标志，草稿和拖动快照也必须阻止交接
        Action::History
            if !busy && editor_state.pending.is_none() && editor_state.ann_drag.is_none() =>
        {
            match crate::features::capture_history::transport::spawn() {
                Ok(()) => editor_state.cancel = true,
                Err(error) => {
                    log::error!("【截图历史】【窗口交接】无法打开历史窗口: {error}");
                    editor_state.toasts.show(
                        crate::ui::toast::ToastKind::HistoryFailed,
                        &mut editor_state.text.font_system,
                    );
                    mark_dirty(dirty_mask, editor_state.input.pointer.monitor_idx);
                }
            }
        }
        Action::History => {}
        Action::SizeUp => step_size(editor_state, StepperArrow::Up, dirty_mask),
        Action::SizeDown => step_size(editor_state, StepperArrow::Down, dirty_mask),
        Action::Tool(tool) if !busy || editor_state.selected_tool.is_drawing() => {
            select_tool(editor_state, tool, dirty_mask)
        }
        Action::Move { dir, fast } if !busy => {
            nudge_selection(editor_state, dir, fast, false, dirty_mask)
        }
        Action::Resize { dir, fast } if !busy => {
            nudge_selection(editor_state, dir, fast, true, dirty_mask)
        }
        Action::Tool(_) | Action::Move { .. } | Action::Resize { .. } => {}
    }
    apply_damage_rects(editor_state, dirty_mask);
}

fn tool_copy(editor_state: &mut EditorState, dirty_mask: &mut u32) -> bool {
    match editor_state.selected_tool {
        Tool::Eyedropper => {
            let color = editor_state.tool_settings.color;
            crate::tools::eyedropper::copy_value(editor_state, ValueField::Hex, color);
            true
        }
        Tool::Ocr if editor_state.ocr.view.is_active() => {
            crate::tools::ocr::copy_selection(editor_state, dirty_mask);
            true
        }
        _ => false,
    }
}

fn select_all(editor_state: &mut EditorState, dirty_mask: &mut u32) {
    if editor_state.selected_tool == Tool::Ocr {
        if editor_state.ocr.view.is_active() {
            crate::tools::ocr::select_all_text(editor_state);
        }
        return;
    }
    let placement = &editor_state.placements[editor_state.input.pointer.monitor_idx];
    let (x, y) = placement.offset();
    editor_state.selection.zone =
        Rect::from_xywh(x, y, placement.size.0 as f32, placement.size.1 as f32);
    mark_all_dirty(dirty_mask, editor_state.placements.len());
    refresh_panels(editor_state, dirty_mask);
}

fn nudge_selection(
    editor_state: &mut EditorState,
    dir: Dir,
    fast: bool,
    resize: bool,
    dirty_mask: &mut u32,
) {
    if editor_state.selected_tool == Tool::Ocr {
        return;
    }
    let (Some(old), Some(bounds)) = (
        editor_state.selection.zone,
        get_full_workspace_rect(&editor_state.placements),
    ) else {
        return;
    };

    let selection = &crate::config::get().selection;
    let step = if fast {
        selection.nudge_fast
    } else {
        selection.nudge
    }
    .round()
    .max(1.0);
    let (dx, dy) = match dir {
        Dir::Left => (-step, 0.0),
        Dir::Right => (step, 0.0),
        Dir::Up => (0.0, -step),
        Dir::Down => (0.0, step),
    };

    let new = if resize {
        let right = (old.right() + dx).clamp(old.left() + 1.0, bounds.right());
        let bottom = (old.bottom() + dy).clamp(old.top() + 1.0, bounds.bottom());
        Rect::from_ltrb(old.left(), old.top(), right, bottom)
    } else {
        let x = (old.left() + dx).clamp(bounds.left(), bounds.right() - old.width());
        let y = (old.top() + dy).clamp(bounds.top(), bounds.bottom() - old.height());
        Rect::from_xywh(x, y, old.width(), old.height())
    };
    let Some(new) = new.filter(|new| *new != old) else {
        return;
    };

    editor_state.selection.zone = Some(new);
    editor_state.damage_rects.push(DamageZone::Global(old));
    editor_state.damage_rects.push(DamageZone::Global(new));
    refresh_panels(editor_state, dirty_mask);
}

/// 【界面】【界面显示】切换 editor_state 的工具界面并关闭隐藏弹层；dirty_mask 返回重绘显示器
fn toggle_ui(editor_state: &mut EditorState, dirty_mask: &mut u32) {
    editor_state.ui_hidden = !editor_state.ui_hidden;
    if editor_state.ui_hidden {
        editor_state.settings_panel.selected = None;
        close_color_popover(editor_state, dirty_mask);
        close_model_popover(editor_state, dirty_mask);
        super::panels::marker::close(editor_state, dirty_mask);
        super::panels::ruler::close(editor_state, dirty_mask);
        super::panels::number::close(editor_state, dirty_mask);
        super::panels::line::close(editor_state, dirty_mask);
        super::panels::filter::close(editor_state, dirty_mask);
    }
    editor_state.toolbar.dirty = true;
    refresh_panels(editor_state, dirty_mask);
}

/// Switch to scrolling capture if a region is selected, or toast otherwise.
fn begin_scroll_capture(editor_state: &mut EditorState, dirty_mask: &mut u32) {
    if editor_state.selection.zone.is_none() {
        editor_state.toasts.show(
            crate::ui::toast::ToastKind::ScrollNeedsRegion,
            &mut editor_state.text.font_system,
        );
        mark_dirty(dirty_mask, editor_state.input.pointer.monitor_idx);
        return;
    }
    editor_state.mode = Mode::Scroll;
    editor_state.cancel = false;
    editor_state.finish = None;
    editor_state.tool_active = false;
}

/// 【光标放大区】【开关更新】为 editor_state 保存明确配置后更新显示，dirty_mask 接收损坏范围，保存失败保留当前开关，无返回值
fn toggle_magnifier(editor_state: &mut EditorState, dirty_mask: &mut u32) {
    // 1. 【光标放大区】【配置保存】原版配置和明确原生开关随操作保存，旧工具偏好继续使用原有存储
    if crate::config::get().startup.selection_loupe.is_some()
        && let Err(error) = crate::config::startup::save_loupe(editor_state.magnifier.hidden)
    {
        log::error!("【光标放大区】【配置保存】保存失败：{error}");
        editor_state.toasts.show_text(
            crate::ui::toast::ToastKind::SettingsFailed,
            "Failed to save settings".into(),
            &mut editor_state.text.font_system,
        );
        mark_dirty(dirty_mask, editor_state.input.pointer.monitor_idx);
        return;
    }
    // 2. 【光标放大区】【阶段更新】开关只影响支持的选择阶段，取色仍使用独立职责
    let was_shown = editor_state.magnifier_shown();
    editor_state.magnifier.hidden = !editor_state.magnifier.hidden;
    if editor_state.magnifier_shown() == was_shown {
        return;
    }
    let Some(mag) = editor_state.magnifier.current.as_ref() else {
        return;
    };
    let monitor_idx = mag.monitor_idx;
    let placement = &editor_state.placements[monitor_idx];
    let rect = magnifier_rect(
        (mag.pos.0 as f32, mag.pos.1 as f32),
        placement.size.0 as f32,
        placement.size.1 as f32,
        editor_state.picking(),
    );
    editor_state.damage_local(monitor_idx, rect);
    mark_dirty(dirty_mask, monitor_idx);
}

fn step_size(editor_state: &mut EditorState, arrow: StepperArrow, dirty_mask: &mut u32) {
    if editor_state.settings_panel.text_layout.is_some() {
        let steps = if arrow == StepperArrow::Up { 1.0 } else { -1.0 };
        super::panels::text::properties::adjust_points(editor_state, steps, dirty_mask);
        return;
    }
    let Some(widget_idx) = editor_state
        .settings_panel
        .widgets
        .iter()
        .position(|w| matches!(w, SettingsWidget::Stepper { .. }))
    else {
        return;
    };
    apply_stepper_arrow_step(editor_state, widget_idx, arrow, dirty_mask);
    let snapshot = editor_state.settings_panel.pre_edit_snapshot.take();
    editor_state.commit_snapshot(snapshot);
    update_settings_panel(editor_state, dirty_mask);
    sync_stepper_edit_text(editor_state, widget_idx);
}
