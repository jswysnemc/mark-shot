//! 【截图】【事件循环】等待输入和后台任务，调度局部重绘

use super::*;

/// Runs the overlay until the user completes or cancels the selection.
/// By the time this returns, the overlay is already closed and capture is complete.
pub(super) fn run_overlay(
    editor_state: &mut EditorState,
    overlay: &mut Box<dyn ScreenOverlay>,
) -> Result<bool, Box<dyn std::error::Error>> {
    let mut dirty_mask: u32 = 0;
    let mut pacing = Pacing::new(editor_state.captures.len());
    let mut annotations_were_hidden = false;
    // wayland on COSMIC unlike KDE doesn't immediately send pointer event
    let mut pointer_wait = Some(Instant::now() + POINTER_GRACE);

    loop {
        let mut timeout = poll_timeout(editor_state);
        if let Some(deadline) = pointer_wait {
            let left = deadline.saturating_duration_since(Instant::now());
            timeout = Some(timeout.map_or(left, |t| t.min(left)));
        }
        // wake up by the fallback even if no other event ever comes
        let waiting = (0..editor_state.captures.len())
            .any(|i| (is_dirty(dirty_mask, i) || pacing.owes_to(i)) && !overlay.ready(i));
        if waiting {
            timeout = Some(timeout.map_or(FRAME_FALLBACK, |t| t.min(FRAME_FALLBACK)));
        }
        poll_background_work(editor_state, &mut dirty_mask);

        match overlay.next_event(timeout)? {
            OverlayEvent::EscapePressed => break,
            ev => {
                match ev {
                    OverlayEvent::PointerMove { .. } => pointer_wait = None,
                    // initiailly toolbar didn't appear as magnifier untill the first pointer event
                    // once again not a conscious chocie, thought it was enough since on kde it always
                    // send pointer event even without moving the pointer if overlay is under it
                    // but cosmic doesn't, so toolbar now appears when the keyboard is focused, which makes
                    // sense even more than pointer
                    OverlayEvent::Focus { monitor_idx } if pointer_wait.is_some() => {
                        editor_state.input.pointer.monitor_idx = monitor_idx;
                    }
                    _ => {}
                }
                handle_event(editor_state, ev, &mut dirty_mask);
            }
        }

        if pointer_wait.is_some_and(|deadline| Instant::now() >= deadline) {
            pointer_wait = None;
            panels::toolbar::update_toolbar(editor_state, &mut dirty_mask);
            apply_damage_rects(editor_state, &mut dirty_mask);
        }

        if editor_state.save_as_requested {
            editor_state.save_as_requested = false;
            if let Some(path) = super::save_as::open(editor_state, overlay.as_mut())? {
                editor_state.saved_as = Some(path);
                return Ok(false);
            }
            // 1. 【截图另存为】【恢复重绘】表面已重建，旧帧回调和局部损坏不能跳过完整背景恢复
            pacing = Pacing::new(editor_state.captures.len());
            mark_all_dirty(&mut dirty_mask, editor_state.captures.len());
            editor_state.annotations_dirty = true;
            pointer_wait = None;
        }

        if editor_state.finish.is_some() || editor_state.cancel {
            break;
        }
        super::text_menu_surface::sync(editor_state, overlay.as_mut(), &mut dirty_mask)?;
        super::text_input::sync(editor_state, overlay.as_mut(), &mut dirty_mask)?;

        // hand off to the scrolling session once a region is selected and mouse is released
        if editor_state.mode == Mode::Scroll
            && editor_state.selection.zone.is_some()
            && !editor_state.input.mouse_down
        {
            return Ok(true);
        }

        overlay.set_cursor(input::compute_cursor(editor_state));
        tick_panels(editor_state, &mut dirty_mask);

        let annotations_hidden = editor_state.selected_tool == Tool::Ocr;
        if annotations_hidden != annotations_were_hidden {
            annotations_were_hidden = annotations_hidden;
            damage_hidden_annotations(editor_state, &mut dirty_mask);
        }

        // pass only when someone can show it
        let drawable = (0..editor_state.captures.len())
            .any(|i| (is_dirty(dirty_mask, i) || pacing.owes_to(i)) && overlay.ready(i));
        if drawable {
            render_dirty_monitors(
                editor_state,
                overlay.as_mut(),
                dirty_mask,
                annotations_hidden,
                &mut pacing,
            )?;
            clear_frame_state(editor_state, &mut dirty_mask, &pacing);
        }
    }

    Ok(false)
}
/// system can't sleep waiting only for events, since we have animation and
/// download beat and etc
pub(super) fn poll_timeout(editor_state: &EditorState) -> Option<Duration> {
    let is_animating = editor_state.toolbar.is_animating()
        || editor_state.action_toolbar.is_animating()
        || editor_state.color_popover.is_animating()
        || editor_state.model_popover.is_animating()
        || editor_state.marker_popover.is_animating()
        || editor_state.ruler_popover.is_animating()
        || editor_state.number_popover.is_animating()
        || editor_state.arrow_popover.is_animating()
        || editor_state.line_popover.is_animating()
        || editor_state.filter_popover.is_animating()
        || editor_state.toasts.is_animating();
    let stepper_holding = editor_state.settings_panel.arrow_held.is_some()
        || editor_state
            .input
            .text_property_press
            .as_ref()
            .is_some_and(|press| press.page != 0);
    let ocr_working = editor_state.ocr.runtime.needs_poll();

    let mut timeout = if is_animating || stepper_holding || ocr_working {
        Some(anim::frame())
    } else if editor_state.ocr.models.is_downloading() {
        Some(DOWNLOAD_POLL)
    } else {
        None
    };
    if let Some(deadline) = editor_state.text_size_history.deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        timeout = Some(timeout.map_or(remaining, |other| other.min(remaining)));
    }
    if let Some(deadline) = editor_state
        .text_session
        .as_ref()
        .and_then(|session| session.caret.deadline())
    {
        let remaining = deadline.saturating_duration_since(Instant::now());
        timeout = Some(timeout.map_or(remaining, |other| other.min(remaining)));
    }
    if editor_state.laser.is_animating() {
        let interval = crate::features::laser_trace::REFRESH_INTERVAL;
        Some(timeout.map_or(interval, |other| other.min(interval)))
    } else {
        timeout
    }
}

pub(super) fn poll_background_work(editor_state: &mut EditorState, dirty_mask: &mut u32) {
    if let Some(fonts) = editor_state.text.system_fonts.take_if(|f| f.is_finished())
        && let Ok(db) = fonts.join()
    {
        crate::fonts::add_system(&mut editor_state.text.font_system, db);
    }
    if let Some(result) = editor_state.ocr.runtime.poll() {
        crate::tools::ocr::finish_ocr(editor_state, result, dirty_mask);
        panels::settings::update_settings_panel(editor_state, dirty_mask);
    }
    panels::model::tick_model_downloads(editor_state, dirty_mask);
    if editor_state.ocr.runtime.is_busy() {
        crate::tools::ocr::tick_scan_badge(editor_state, dirty_mask);
    }
}

pub(super) fn handle_event(editor_state: &mut EditorState, ev: OverlayEvent, dirty_mask: &mut u32) {
    match ev {
        ev @ (OverlayEvent::TextInputFocus(_)
        | OverlayEvent::TextInputLeave { .. }
        | OverlayEvent::TextInput(_)) => {
            super::text_input::event(editor_state, ev, dirty_mask);
        }
        OverlayEvent::Tick | OverlayEvent::EscapePressed | OverlayEvent::FrameDone { .. } => {}
        OverlayEvent::Focus { monitor_idx } => {
            editor_state.input.keyboard_focus = Some(monitor_idx)
        }
        OverlayEvent::FocusLost { monitor_idx } => {
            if editor_state.input.keyboard_focus == Some(monitor_idx) {
                editor_state.input.keyboard_focus = None;
            }
        }
        OverlayEvent::PointerMove { monitor_idx, x, y } => {
            input::handle_pointer_move(editor_state, monitor_idx, x, y, dirty_mask);
        }
        OverlayEvent::PointerButton {
            button,
            pressed,
            serial,
        } => {
            let text_pointer = matches!(button, crate::types::MouseButton::Left)
                && (crate::tools::text::session_input::inside(editor_state)
                    || editor_state
                        .text_session
                        .as_ref()
                        .is_some_and(|session| session.dragging));
            if pressed {
                editor_state.input.popup_serial = serial;
            }
            input::handle_pointer_button(editor_state, button, pressed, dirty_mask);
            if text_pointer {
                super::text_caret::pointer_activity(editor_state, Instant::now(), dirty_mask);
            }
        }
        OverlayEvent::Popup { id, event } => {
            super::text_menu_surface::event(editor_state, id, event, dirty_mask)
        }
        OverlayEvent::Key { chord, text } => {
            input::handle_key(editor_state, chord, text, dirty_mask);
            super::text_caret::activity(editor_state, dirty_mask);
        }
        OverlayEvent::ModifiersChanged { ctrl, shift } => {
            input::handle_modifiers(editor_state, ctrl, shift, dirty_mask);
        }
        OverlayEvent::Scroll(event) => {
            input::handle_axis_scroll(editor_state, event, dirty_mask);
        }
    }
}

/// tick animation
pub(super) fn tick_panels(editor_state: &mut EditorState, dirty_mask: &mut u32) {
    super::text_caret::tick(editor_state, Instant::now(), dirty_mask);
    editor_state.tick_text_size_wheel(Instant::now());
    panels::text::properties::tick(editor_state, Instant::now(), dirty_mask);
    tick_panel_animation(
        &mut editor_state.action_toolbar,
        &mut editor_state.damage_rects,
        dirty_mask,
    );
    tick_panel_animation(
        &mut editor_state.toolbar,
        &mut editor_state.damage_rects,
        dirty_mask,
    );
    tick_panel_animation(
        &mut editor_state.color_popover,
        &mut editor_state.damage_rects,
        dirty_mask,
    );
    tick_panel_animation(
        &mut editor_state.model_popover,
        &mut editor_state.damage_rects,
        dirty_mask,
    );
    tick_panel_animation(
        &mut editor_state.marker_popover,
        &mut editor_state.damage_rects,
        dirty_mask,
    );
    tick_panel_animation(
        &mut editor_state.ruler_popover,
        &mut editor_state.damage_rects,
        dirty_mask,
    );
    tick_panel_animation(
        &mut editor_state.number_popover,
        &mut editor_state.damage_rects,
        dirty_mask,
    );
    tick_panel_animation(
        &mut editor_state.arrow_popover,
        &mut editor_state.damage_rects,
        dirty_mask,
    );
    tick_panel_animation(
        &mut editor_state.line_popover,
        &mut editor_state.damage_rects,
        dirty_mask,
    );
    tick_panel_animation(
        &mut editor_state.filter_popover,
        &mut editor_state.damage_rects,
        dirty_mask,
    );
    panels::settings::tick_stepper_arrow_hold(editor_state, dirty_mask);

    let toast_place = {
        let idx = editor_state.input.pointer.monitor_idx;
        let placement = &editor_state.placements[idx];
        crate::ui::toast::ToastPlace {
            monitor_idx: idx,
            size: (placement.size.0 as f32, placement.size.1 as f32),
            focus: editor_state
                .selection
                .zone
                .as_ref()
                .and_then(|zone| global_selection_to_local(zone, placement)),
        }
    };
    editor_state
        .toasts
        .tick(toast_place, &mut editor_state.damage_rects, dirty_mask);

    crate::tools::laser::tick(editor_state, dirty_mask, Instant::now());

    if editor_state.toolbar.is_animating()
        || editor_state.action_toolbar.is_animating()
        || editor_state.color_popover.is_animating()
        || editor_state.model_popover.is_animating()
        || editor_state.marker_popover.is_animating()
        || editor_state.ruler_popover.is_animating()
        || editor_state.number_popover.is_animating()
        || editor_state.arrow_popover.is_animating()
        || editor_state.line_popover.is_animating()
        || editor_state.filter_popover.is_animating()
    {
        if editor_state.settings_panel.visible {
            panels::settings::update_settings_panel(editor_state, dirty_mask);
        }
        panels::text::font_popup::update(editor_state, dirty_mask);
        if editor_state.color_popover.is_visible() {
            panels::color::update_color_popover(editor_state, dirty_mask);
        }
        if editor_state.model_popover.is_visible() {
            panels::model::update_model_popover(editor_state, dirty_mask);
        }
        if editor_state.marker_popover.is_visible() {
            panels::marker::update(editor_state, dirty_mask);
        }
        if editor_state.ruler_popover.is_visible() {
            panels::ruler::update(editor_state, dirty_mask);
        }
        if editor_state.number_popover.is_visible() {
            panels::number::update(editor_state, dirty_mask);
        }
        if editor_state.arrow_popover.is_visible() || editor_state.line_popover.is_visible() {
            panels::line::update(editor_state, dirty_mask);
        }
        if editor_state.filter_popover.is_visible() {
            panels::filter::update(editor_state, dirty_mask);
        }
    }
}

// ocr tool hides annotations, so need to mark diryt everything
pub(super) fn damage_hidden_annotations(editor_state: &mut EditorState, dirty_mask: &mut u32) {
    let has_annotations = !editor_state.annotations.is_empty()
        || editor_state.pending_pen_baked != 0
        || !editor_state.laser.is_empty();
    if has_annotations && let Some(workspace) = get_full_workspace_rect(&editor_state.placements) {
        editor_state
            .damage_rects
            .push(crate::editor::DamageZone::Global(workspace));
        mark_all_dirty(dirty_mask, editor_state.placements.len());
    }
}
