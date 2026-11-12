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

    let mut visibility = super::capture_visibility::Visibility::new(overlay.as_mut())?;
    loop {
        let (hidden, restored) = visibility.sync(editor_state, overlay.as_mut())?;
        if hidden {
            overlay.next_event(Some(Duration::from_millis(100)))?;
            continue;
        }
        if restored {
            pacing = Pacing::new(editor_state.captures.len());
            mark_all_dirty(&mut dirty_mask, editor_state.captures.len());
            editor_state.annotations_dirty = true;
        }
        let mut timeout = poll_timeout(editor_state);
        if let Some(left) = pacing.source_residency.timeout(Instant::now()) {
            timeout = Some(timeout.map_or(left, |timeout| timeout.min(left)));
        }
        if let Some(left) = pacing
            .foreground_residency
            .timeout(Instant::now(), |monitor| {
                !is_dirty(dirty_mask, monitor) && !pacing.owes_to(monitor)
            })
        {
            timeout = Some(timeout.map_or(left, |timeout| timeout.min(left)));
        }
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

        let event = overlay.next_event(timeout)?;
        if let Some(size) = overlay.image_window_size()
            && editor_state.placements[0].size != (size.0 as i32, size.1 as i32)
        {
            super::image_window::resize(editor_state, size, &mut dirty_mask);
            pacing = Pacing::new(editor_state.captures.len());
        }
        match event {
            OverlayEvent::EscapePressed if editor_state.color_inspector.panel.is_some() => {
                crate::features::color_sample::inspector::close(editor_state, &mut dirty_mask);
            }
            OverlayEvent::EscapePressed if editor_state.selection_size.interacting() => {
                super::selection_size::keyboard::key(
                    editor_state,
                    Some(crate::keys::Chord {
                        key: crate::keys::Key::Escape,
                        mods: Default::default(),
                    }),
                    None,
                    &mut dirty_mask,
                );
            }
            OverlayEvent::EscapePressed
                if editor_state.selection_translation.busy()
                    || editor_state.selection_translation.popup.is_some()
                    || editor_state.selection_translation.session.is_some() =>
            {
                super::selection_translation::input::key(
                    editor_state,
                    Some(crate::keys::Chord {
                        key: crate::keys::Key::Escape,
                        mods: Default::default(),
                    }),
                    &mut dirty_mask,
                );
            }
            OverlayEvent::EscapePressed if editor_state.upload.busy() => {
                super::upload::cancel(editor_state, &mut dirty_mask)
            }
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
                super::cursor_magnifier::sync(editor_state, &mut dirty_mask);
            }
        }
        super::region_detection::task::poll(editor_state, &mut dirty_mask);
        super::capture_start::sync(editor_state, &mut dirty_mask);
        if let Some(action) = editor_state.image_window.action.take() {
            if let Err(error) = overlay.image_window_action(action, editor_state.input.popup_serial)
            {
                log::error!("【图片窗口】【系统交互】{error}");
            }
            if matches!(
                action,
                crate::types::image_window::Action::Move
                    | crate::types::image_window::Action::Resize(_)
            ) {
                editor_state.image_window.pressed = None;
            }
        }
        super::region_detection::input::sync(editor_state, &mut dirty_mask);
        super::upload::start(editor_state, overlay.as_mut(), &mut dirty_mask);
        super::annotation_selection::sync(editor_state, &mut dirty_mask);
        super::upload::poll(editor_state, &mut dirty_mask);

        if pointer_wait.is_some_and(|deadline| Instant::now() >= deadline) {
            pointer_wait = None;
            panels::toolbar::update_toolbar(editor_state, &mut dirty_mask);
            apply_damage_rects(editor_state, &mut dirty_mask);
        }

        if editor_state.preferences_requested {
            editor_state.preferences_requested = false;
            crate::editor::text_session::commit(editor_state, &mut dirty_mask);
            overlay.suspend()?;
            visibility.pause();
            let before = crate::config::preferences::current();
            let result = crate::features::preferences::run(false);
            overlay.resume()?;
            crate::features::preferences::runtime::apply(
                editor_state,
                &before,
                &crate::config::preferences::current(),
                &mut dirty_mask,
            );
            if let Err(error) = result {
                log::error!("【设置窗口】【编辑器返回】{error}");
                editor_state.toasts.show_text(
                    crate::ui::toast::ToastKind::SettingsFailed,
                    error.to_string(),
                    &mut editor_state.text.font_system,
                );
            }
            super::theme_switch::refresh(editor_state, &mut dirty_mask);
            pacing = Pacing::new(editor_state.captures.len());
            editor_state.annotations_dirty = true;
            pointer_wait = None;
        }

        if editor_state.save_as_requested {
            editor_state.save_as_requested = false;
            overlay.suspend()?;
            visibility.pause();
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
            if !editor_state.cancel {
                super::selection_history::record(editor_state);
            }
            break;
        }
        if let Some(action) = editor_state.external_action_requested.take() {
            if super::external_actions::start(editor_state, overlay.as_mut(), action)? {
                editor_state.cancel = true;
                break;
            }
            pacing = Pacing::new(editor_state.captures.len());
            mark_all_dirty(&mut dirty_mask, editor_state.captures.len());
            pointer_wait = None;
        }
        if !super::selection_translation::panels::sync(editor_state, overlay.as_mut())?
            && !super::selection_size::popup::sync(editor_state, overlay.as_mut())?
        {
            super::text_menu_surface::sync(editor_state, overlay.as_mut(), &mut dirty_mask)?;
        }
        if editor_state.selection_translation.requested.is_some()
            && crate::ocr::engine_choice::source(&editor_state.ocr.models).is_none()
        {
            overlay.suspend()?;
            visibility.pause();
        }
        if super::selection_translation::tasks::start(
            editor_state,
            overlay.as_mut(),
            &mut dirty_mask,
        )? {
            pacing = Pacing::new(editor_state.captures.len());
            mark_all_dirty(&mut dirty_mask, editor_state.captures.len());
            pointer_wait = None;
        }
        super::selection_translation::tasks::poll(editor_state, overlay.as_mut(), &mut dirty_mask);
        super::selection_translation::panels::sync(editor_state, overlay.as_mut())?;
        super::selection_size::field_clipboard::sync(editor_state, overlay.as_mut());
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
        pacing.source_residency.reclaim_due(
            editor_state.file_canvas.as_ref().map(|file| &file.original),
            Instant::now(),
        );
        super::foreground_residency::reclaim(editor_state, &mut pacing, dirty_mask, Instant::now());
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
    } else if editor_state.ocr.models.needs_poll() {
        Some(DOWNLOAD_POLL)
    } else {
        None
    };
    if let Some(interval) = editor_state.selection_translation.poll_interval() {
        timeout = Some(timeout.map_or(interval, |other| other.min(interval)));
    }
    if editor_state.capture_start.visible {
        let interval = editor_state.capture_start.query.timeout();
        timeout = Some(timeout.map_or(interval, |other| other.min(interval)));
    }
    if let Some(deadline) = editor_state.selection_translation.ocr_deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        timeout = Some(timeout.map_or(remaining, |other| other.min(remaining)));
    }
    if let Some(preview) = &editor_state.navigation.preview {
        let remaining = preview.deadline.saturating_duration_since(Instant::now());
        timeout = Some(timeout.map_or(remaining, |other| other.min(remaining)));
    }
    if let Some(deadline) = editor_state.text_size_history.deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        timeout = Some(timeout.map_or(remaining, |other| other.min(remaining)));
    }
    if let Some(deadline) = editor_state.upload.deadline() {
        let remaining = deadline.saturating_duration_since(Instant::now());
        timeout = Some(timeout.map_or(remaining, |other| other.min(remaining)));
    }
    if let Some(deadline) = editor_state.toasts.deadline() {
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
    if let Some(deadline) = editor_state.selection_size.caret.deadline() {
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
    super::capture_start::poll(editor_state, dirty_mask);
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
    if super::selection_size::field_clipboard::event(editor_state, &ev, dirty_mask) {
        return;
    }
    if super::upload::event(editor_state, &ev, dirty_mask) {
        return;
    }
    match ev {
        OverlayEvent::ThemeChanged => super::theme_switch::refresh(editor_state, dirty_mask),
        OverlayEvent::PopupSerial(serial) => editor_state.input.popup_serial = Some(serial),
        ev @ (OverlayEvent::TextInputFocus(_)
        | OverlayEvent::TextInputLeave { .. }
        | OverlayEvent::TextInput(_)) => {
            super::text_input::event(editor_state, ev, dirty_mask);
        }
        OverlayEvent::Tick
        | OverlayEvent::CanvasResized { .. }
        | OverlayEvent::EscapePressed
        | OverlayEvent::FrameDone { .. }
        | OverlayEvent::ScrollPanelConfigured { .. }
        | OverlayEvent::PrimaryText { .. } => {}
        OverlayEvent::Focus { monitor_idx } => {
            editor_state.input.keyboard_focus = Some(monitor_idx)
        }
        OverlayEvent::FocusLost { monitor_idx } => {
            editor_state.capture_start.pressed = None;
            super::canvas_navigation::input::focus_lost(editor_state, dirty_mask);
            super::annotation_collection::input::focus_lost(editor_state, dirty_mask);
            super::selection_size::apply::commit(editor_state, dirty_mask);
            super::external_actions::close(editor_state, dirty_mask);
            if editor_state.input.keyboard_focus == Some(monitor_idx) {
                editor_state.input.keyboard_focus = None;
            }
        }
        OverlayEvent::PointerMove {
            monitor_idx,
            x,
            y,
            history,
        } => {
            super::region_detection::motion::history(editor_state, &history);
            // 1. 【选区尺寸】【移动轨迹】极值先转换为全局坐标，移出后返回也必须终止平台双击候选
            for range in &history {
                if let Some(output) = editor_state.placements.get(range.monitor_idx).cloned() {
                    for point in [range.min, range.max] {
                        let point = (
                            f64::from(output.position.0) + point.0,
                            f64::from(output.position.1) + point.1,
                        );
                        editor_state.selection_size.clicks.moved(point);
                        input::capture_actions::observe(editor_state, point);
                    }
                }
            }
            super::pointer_history::dispatch(editor_state, &history, monitor_idx, x, y, dirty_mask);
        }
        OverlayEvent::PointerButton {
            button,
            pressed,
            serial,
            timestamp,
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
            editor_state.input.pointer_timestamp = timestamp;
            input::handle_pointer_button(editor_state, button, pressed, dirty_mask);
            editor_state.input.pointer_timestamp = None;
            if text_pointer {
                super::text_caret::pointer_activity(editor_state, Instant::now(), dirty_mask);
            }
        }
        OverlayEvent::Popup { id, event } => {
            if !super::selection_translation::input::popup(
                editor_state,
                id,
                event.clone(),
                dirty_mask,
            ) && !super::selection_size::popup::event(
                editor_state,
                id,
                event.clone(),
                dirty_mask,
            ) {
                super::text_menu_surface::event(editor_state, id, event, dirty_mask)
            }
        }
        OverlayEvent::Key { chord, text } => {
            super::canvas_navigation::preview::clear(editor_state, dirty_mask);
            input::handle_key(editor_state, chord, text, dirty_mask);
            super::text_caret::activity(editor_state, dirty_mask);
        }
        OverlayEvent::ModifiersChanged { ctrl, shift } => {
            input::handle_modifiers(editor_state, ctrl, shift, dirty_mask);
        }
        OverlayEvent::Scroll(event) => {
            if !editor_state.selection_translation.busy()
                && editor_state.selection_translation.popup.is_none()
            {
                input::handle_axis_scroll(editor_state, event, dirty_mask);
            }
        }
    }
    super::selection_size::panels::update(editor_state, dirty_mask);
    super::selection_translation::panels::update(editor_state, dirty_mask);
}

/// tick animation
pub(super) fn tick_panels(editor_state: &mut EditorState, dirty_mask: &mut u32) {
    super::canvas_navigation::preview::tick(editor_state, Instant::now(), dirty_mask);
    super::selection_size::caret::tick(editor_state, Instant::now(), dirty_mask);
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
