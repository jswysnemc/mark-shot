use crate::editor::dirty::mark_all_dirty;
use crate::editor::{DamageZone, EditorState};
use crate::interaction::ClickTarget;
use crate::ocr::draw::scan_badge_rect;
use crate::ocr::models::MODELS;
use crate::ocr::{self, StartOutcome};
use crate::tools::selection::SelectionTool;
use crate::tools::{Tool, ToolBehavior};
use crate::types::{CursorIcon, MouseButton, SelectionHandle, SpecialKey};
use crate::ui::toast::ToastKind;
use log::{error, info};
use std::time::Instant;
use tiny_skia::Rect;

pub struct OcrTool;

impl ToolBehavior for OcrTool {
    // Picking the tool recognizes straight away when a selection is already
    // there. The work runs on a background thread; `app` polls for the result
    // and calls `finish_ocr`.
    fn on_activate(&self, state: &mut EditorState, _dirty_mask: &mut u32) {
        state.ocr.runtime.prepare();
        if state.ocr.runtime.is_busy() {
            return;
        }

        damage_all(state);

        // If this selection zone is already scanned, shows older result without recanning
        if state.ocr.view.is_active() && state.ocr.view.region() == state.selection.zone {
            return;
        }
        state.ocr.view.clear();

        if state.ocr.models.installed_count() == 0 {
            ask_for_model(state);
            return;
        }

        // if there is no selection, ask for dragging one, using notification toast
        if state.selection.zone.is_none() {
            await_region(state);
            return;
        }
        start_ocr(state);
    }

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
        state.input.mouse_down = pressed;

        if !pressed {
            end_region_drag(state);
            return;
        }

        let hit = state.ocr.view.line_at(state.canvas_point());
        if let Some(line) = hit {
            let pos = (state.canvas_point().0 as f32, state.canvas_point().1 as f32);
            if state.input.clicks.register(ClickTarget::OcrLine(line), pos) {
                let damage = state.ocr.view.select_block(line);
                damage_overlay(state, damage);
                return;
            }
        }

        // Empty space inside the scanned area still starts a text drag
        if hit.is_some() || pressed_inside_region(state) {
            let damage = state.ocr.view.begin_drag(state.canvas_point());
            damage_overlay(state, damage);
            return;
        }

        let damage = state.ocr.view.deselect();
        damage_overlay(state, damage);
        begin_region_drag(state);
    }

    fn on_move(&self, state: &mut EditorState, global: (f64, f64), dirty_mask: &mut u32) {
        if state.ocr.redrag {
            SelectionTool.on_move(state, global, dirty_mask);
            if boxed_out(state) && state.ocr.view.is_active() {
                // A real drag, not a stray click: the old result no longer
                // describes what is on screen, so take it down now rather than
                // leaving stale plates floating over the new box.
                damage_all(state);
                state.ocr.view.clear();
            }
            return;
        }

        if !state.input.mouse_down || !state.ocr.view.is_active() {
            return;
        }

        let damage = state.ocr.view.extend_drag(global);
        damage_overlay(state, damage);
    }

    fn on_key(&self, state: &mut EditorState, key: SpecialKey, dirty_mask: &mut u32) {
        if !state.ocr.view.is_active() || !state.input.ctrl {
            return;
        }
        match key {
            SpecialKey::KeyA => select_all_text(state),
            SpecialKey::KeyC | SpecialKey::KeyX => copy_selection(state, dirty_mask),
            _ => {}
        }
    }

    // on deactivate we cancel scan in progress, or cache scanned text if it was done.
    // and in both cases clear the visuals
    fn on_deactivate(&self, state: &mut EditorState, _dirty_mask: &mut u32) {
        let scanning = state.ocr.runtime.is_busy();
        cancel_scan(state);
        if scanning {
            state.ocr.view.clear();
        } else {
            let _ = state.ocr.view.deselect();
        }
        state.ocr.redrag = false;
        state.tool_active = false;
        state.ocr.await_region = false;
        state.model_popover.open = false;
        state.toasts.dismiss(ToastKind::OcrPickRegion);
        state.toasts.dismiss(ToastKind::OcrNoModel);
        dismiss_scan_toasts(state);
    }

    fn cursor(&self, state: &EditorState) -> CursorIcon {
        if state.ocr.view.is_active() && state.ocr.view.line_at(state.canvas_point()).is_some() {
            CursorIcon::Text
        } else {
            CursorIcon::Crosshair
        }
    }
}

fn cancel_scan(state: &mut EditorState) {
    state.ocr.runtime.cancel();
    state.ocr.scan_started = None;
    damage_all(state);
}

/// Takes down what the previous scan said, it no longer describes the screen.
fn dismiss_scan_toasts(state: &mut EditorState) {
    for kind in [
        ToastKind::OcrNoText,
        ToastKind::OcrFailed,
        ToastKind::CopyFailed,
    ] {
        state.toasts.dismiss(kind);
    }
}

/// waiting for drag with showing toast
fn await_region(state: &mut EditorState) {
    state.ocr.await_region = true;
    state
        .toasts
        .show(ToastKind::OcrPickRegion, &mut state.text.font_system);
}

fn ask_for_model(state: &mut EditorState) {
    state.model_popover.open = true;
    state
        .toasts
        .show(ToastKind::OcrNoModel, &mut state.text.font_system);
}

fn boxed_out(state: &EditorState) -> bool {
    let min_region = crate::config::get().ocr.min_region;
    state.selection.zone.is_some_and(|zone| {
        Some(zone) != state.ocr.redrag_from
            && zone.width() >= min_region
            && zone.height() >= min_region
    })
}

/// Checks if the click was inside the scanned area. Clicking slightly outside
/// the text should not clear the result, so only a click far outside means
/// "select a new area".
fn pressed_inside_region(state: &EditorState) -> bool {
    state.ocr.view.region().is_some_and(|region| {
        let (x, y) = (state.canvas_point().0 as f32, state.canvas_point().1 as f32);
        x >= region.left() && x <= region.right() && y >= region.top() && y <= region.bottom()
    })
}

/// Start dragging to select a new area. Unlike the main selection tool, this
/// never grabs edges or handles.
///
/// In the OCR tool, dragging anywhere on empty space always starts a new selection.
/// This prevents accidentally moving a full-screen box off the monitor.
fn begin_region_drag(state: &mut EditorState) {
    if state.ocr.runtime.is_busy() {
        cancel_scan(state);
        state.ocr.view.clear();
    }

    state.toasts.dismiss(ToastKind::OcrPickRegion);
    dismiss_scan_toasts(state);
    state.ocr.redrag = true;
    state.ocr.redrag_from = state.selection.zone;
    state.tool_active = true;
    state.selection.set_drag(SelectionHandle::None, None, None);
    state.input.drag_start = Some(state.canvas_point());
}

fn end_region_drag(state: &mut EditorState) {
    if !state.ocr.redrag {
        return;
    }
    state.ocr.redrag = false;
    state.tool_active = false;
    state.input.drag_start = None;
    state.selection.set_drag(SelectionHandle::None, None, None);

    if boxed_out(state) {
        state.ocr.await_region = false;
        start_ocr(state);
        return;
    }

    if state.selection.zone != state.ocr.redrag_from {
        for zone in [state.selection.zone, state.ocr.redrag_from]
            .into_iter()
            .flatten()
        {
            state.damage_rects.push(DamageZone::Global(zone));
        }
        state.selection.zone = state.ocr.redrag_from;
    }

    if state.ocr.await_region {
        await_region(state);
    }
}

/// Redraw the entire overlay: all covered areas, plus the progress badge if
/// a scan is active (since the background behind the badge must be redrawn too).
fn damage_all(state: &mut EditorState) {
    let mut rects: Vec<Rect> = state.ocr.view.bounds().into_iter().collect();
    if let Some(region) = state.ocr.view.region() {
        rects.push(scan_badge_rect(region));
    }
    for rect in rects {
        state.damage_rects.push(DamageZone::Global(rect));
    }
}

/// Redraw just the area an interaction changed. The overlay redraws itself
/// clipped to this, so nothing else on screen is touched.
pub fn select_all_text(state: &mut EditorState) {
    let damage = state.ocr.view.select_all();
    damage_overlay(state, damage);
}

fn damage_overlay(state: &mut EditorState, rect: Option<Rect>) {
    if let Some(rect) = rect {
        state.damage_rects.push(DamageZone::Global(rect));
    }
}

fn start_ocr(state: &mut EditorState) {
    let Some(region) = state.selection.zone else {
        return;
    };
    dismiss_scan_toasts(state);

    if !state.ocr.models.active_installed() {
        ask_for_model(state);
        return;
    }

    let Some(prepared) = ocr::preprocess::region(state, region) else {
        return;
    };

    match state.ocr.runtime.start(prepared.image) {
        StartOutcome::Started => {
            state.ocr.result_scale = prepared.scale;
            state.ocr.view.set_region(region);
            state.ocr.scan_started = Some(Instant::now());
            // The shade and the badge both go up on the next frame.
            state.damage_rects.push(DamageZone::Global(region));
        }
        StartOutcome::Busy => info!("ocr: still working on the previous region"),
        StartOutcome::Unavailable(e) => {
            error!("ocr: engine unavailable: {e}");
            if e.starts_with(ocr::model_integrity::IntegrityError::PREFIX)
                && state.ocr.models.repair_active()
            {
                ask_for_model(state);
            }
            state
                .toasts
                .show(ToastKind::OcrFailed, &mut state.text.font_system);
        }
    }
}

/// Re-run recognition over the current selection. Driven by the panel's rescan
/// button; another area is picked by dragging a new box instead.
pub fn restart_ocr(state: &mut EditorState, _dirty_mask: &mut u32) {
    damage_all(state);
    state.ocr.view.clear();
    start_ocr(state);
}

/// Copy what is selected, or everything when nothing is.
pub fn copy_selection(state: &mut EditorState, _dirty_mask: &mut u32) {
    let text = state.ocr.view.text_to_copy();
    if text.is_empty() {
        return;
    }
    match crate::utils::copy_to_clipboard(&text) {
        Ok(()) => info!("ocr: copied {} line(s)", text.lines().count()),
        Err(e) => {
            info!("ocr: can't copy: {e}");
            state
                .toasts
                .show(ToastKind::CopyFailed, &mut state.text.font_system);
        }
    }
}

/// Select everything, then copy it, so the panel's copy button also shows what
/// it took.
pub fn copy_all(state: &mut EditorState, dirty_mask: &mut u32) {
    let damage = state.ocr.view.select_all();
    damage_overlay(state, damage);
    copy_selection(state, dirty_mask);
}

/// One animation step of the progress badge while recognition runs. Only the
/// badge is damaged
pub fn tick_scan_badge(state: &mut EditorState, dirty_mask: &mut u32) {
    let Some(region) = state.ocr.view.region() else {
        return;
    };
    let badge = scan_badge_rect(region);
    state.damage_rects.push(DamageZone::Global(badge));
    *dirty_mask |= crate::utils::get_overlapping_monitors(&badge, &state.placements);
}

/// Called from the event loop when a recognition finishes: load the lines into
/// the view so they can be selected and copied.
pub fn finish_ocr(
    state: &mut EditorState,
    result: Result<ocr::OcrText, String>,
    dirty_mask: &mut u32,
) {
    state.ocr.scan_started = None;
    // Whatever happens next, the progress badge has to come off the canvas.
    damage_all(state);

    let text = match result {
        Ok(text) => text,
        Err(e) => {
            error!("ocr: recognition failed: {e}");
            if e.starts_with(ocr::model_integrity::IntegrityError::PREFIX)
                && state.ocr.models.repair_active()
            {
                ask_for_model(state);
            }
            state
                .toasts
                .show(ToastKind::OcrFailed, &mut state.text.font_system);
            state.ocr.view.clear();
            mark_all_dirty(dirty_mask, state.placements.len());
            return;
        }
    };

    if text.is_empty() {
        info!("ocr: no text found in the selected region");
        state
            .toasts
            .show(ToastKind::OcrNoText, &mut state.text.font_system);
        // Nothing to shade or select: drop the overlay rather than leaving the
        // region sitting under a wash with no text in it.
        state.ocr.view.clear();
        mark_all_dirty(dirty_mask, state.placements.len());
        return;
    }

    let text = ocr::preprocess::normalize(text, state.ocr.result_scale);
    state.ocr.view.set_lines(text.lines);
    info!("ocr: {} line(s)", state.ocr.view.lines.len());

    if state.ocr.copy_on_completion {
        copy_all(state, dirty_mask);
    }
    if state.ocr.open_result_on_completion {
        open_result(state);
    }

    damage_all(state);
    mark_all_dirty(dirty_mask, state.placements.len());
}

/// 【文字识别】【结果窗口】把 state 当前识别结果交给独立窗口；成功后退出覆盖层且不导出截图
pub fn open_result(state: &mut EditorState) {
    let result = crate::features::ocr_result::document::OcrDocument::from_editor(state)
        .and_then(|document| crate::features::ocr_result::transport::spawn(&document));
    match result {
        Ok(()) => state.cancel = true,
        Err(error) => {
            error!("【文字识别】【结果窗口】无法打开结果窗口: {error}");
            state
                .toasts
                .show(ToastKind::OcrFailed, &mut state.text.font_system);
        }
    }
}

/// after downloading starts reading immediately
pub fn use_model(state: &mut EditorState, idx: usize, dirty_mask: &mut u32) {
    if state.ocr.models.active() == Some(idx) {
        return;
    }
    let Some(files) = state.ocr.models.files(idx) else {
        return;
    };
    state.ocr.models.set_active(Some(idx));
    state.ocr.runtime.load(files);

    if state.selected_tool != Tool::Ocr {
        state.ocr.view.clear();
        return;
    }
    state.ocr.runtime.prepare();
    state.model_popover.open = false;
    if state.selection.zone.is_some() || state.ocr.runtime.is_busy() || state.ocr.view.is_active() {
        cancel_scan(state);
        restart_ocr(state, dirty_mask);
    } else {
        await_region(state);
    }
}

pub fn model_ready(state: &mut EditorState, idx: usize, dirty_mask: &mut u32) {
    if state.ocr.models.active() == Some(idx) {
        if let Some(files) = state.ocr.models.files(idx) {
            state.ocr.runtime.load(files);
            if state.selected_tool == Tool::Ocr {
                state.ocr.runtime.prepare();
            }
        }
    } else if !state.ocr.models.active_installed() {
        use_model(state, idx, dirty_mask);
    }

    if state.ocr.models.installed_count() != 1 || state.selected_tool != Tool::Ocr {
        return;
    }
    state.model_popover.open = false;
    state.toasts.dismiss(ToastKind::OcrNoModel);
    if state.selection.zone.is_some() {
        start_ocr(state);
    } else {
        await_region(state);
    }
}

pub fn model_failed(state: &mut EditorState, idx: usize, err: &str) {
    error!("ocr: failed to download {}: {err}", MODELS[idx].name);
    if state.selected_tool == Tool::Ocr {
        state
            .toasts
            .show(ToastKind::OcrDownloadFailed, &mut state.text.font_system);
    }
}

pub fn cancel_model_download(state: &mut EditorState, idx: usize) {
    state.ocr.models.cancel(idx);
    if state.ocr.models.active() == Some(idx) {
        state.ocr.models.set_active(None);
    }
}
