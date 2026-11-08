// popover with list of model languages

use crate::editor::EditorState;
use crate::editor::dirty::apply_damage_rects;
use crate::ocr::models::{MODELS, ModelEvent, ModelStatus};
use crate::ui::model_popover::{ModelPopoverElement, ModelRow};
use crate::ui::panel::{UiPanel, emit_panel_damage, sync_panel_hover, sync_panel_rect};
use crate::ui::settings_panel::{SettingsAction, SettingsWidget};
use crate::ui::toast::ToastKind;

use super::settings::{compute_popover_placement, update_settings_panel};

pub fn update_model_popover(editor_state: &mut EditorState, dirty_mask: &mut u32) {
    let old_rect = editor_state.model_popover.rect();
    let old_monitor = editor_state.model_popover.monitor_idx;
    let setup_changed = crate::ui::model_popover::setup::sync(
        &mut editor_state.model_popover,
        &editor_state.ocr.models,
    );

    if editor_state.model_popover.is_visible() {
        if setup_changed || editor_state.model_popover.pixmap.is_none() {
            crate::ui::model_popover::setup::prepare_text(
                &mut editor_state.text.font_system,
                &editor_state.model_popover,
            );
        }
        let size = editor_state.model_popover.size;
        let (pos, monitor_idx) =
            compute_popover_placement(editor_state, size, crate::theme::size::margin());
        editor_state.model_popover.position = pos;
        editor_state.model_popover.render_pos = pos;
        editor_state.model_popover.monitor_idx = monitor_idx;
    }

    let models = &editor_state.ocr.models;
    let no_model = models.installed_count() == 0;
    let rows = (0..MODELS.len()).map(|idx| ModelRow {
        status: models.status(idx),
        active: models.active() == Some(idx),
        size: models.download_size(idx),
        recommended: no_model && models.recommended() == idx,
    });
    let rows_changed = editor_state.model_popover.sync_rows(rows) || setup_changed;

    sync_panel_rect(
        &mut editor_state.model_popover,
        old_rect,
        old_monitor,
        rows_changed,
        &mut editor_state.damage_rects,
        dirty_mask,
    );

    if editor_state.model_popover.rect().is_some() {
        let hovered = editor_state
            .model_popover
            .element_at(editor_state.input.pointer.local);
        sync_panel_hover(
            &mut editor_state.model_popover,
            hovered,
            &mut editor_state.damage_rects,
            dirty_mask,
        );
    }

    sync_languages_button(editor_state, dirty_mask);
}

fn sync_languages_button(editor_state: &mut EditorState, dirty_mask: &mut u32) {
    let panel = &mut editor_state.settings_panel;
    let Some(idx) = panel.widgets.iter().position(|w| {
        matches!(
            w,
            SettingsWidget::Action {
                action: SettingsAction::OcrLanguages,
                ..
            }
        )
    }) else {
        return;
    };

    let selected = if editor_state.model_popover.open {
        Some(idx)
    } else {
        panel.selected.filter(|&s| s != idx)
    };
    if panel.selected == selected {
        return;
    }
    panel.selected = selected;
    panel.dirty = true;
    if let Some(rect) = panel.rect() {
        emit_panel_damage(
            rect,
            panel.monitor_idx,
            &mut editor_state.damage_rects,
            dirty_mask,
        );
    }
}

pub fn toggle_model_popover(editor_state: &mut EditorState, dirty_mask: &mut u32) {
    super::text::font_popup::close(editor_state, dirty_mask);
    if !editor_state.model_popover.open {
        super::line::close(editor_state, dirty_mask);
        super::filter::close(editor_state, dirty_mask);
    }
    editor_state.model_popover.open = !editor_state.model_popover.open;
    update_model_popover(editor_state, dirty_mask);
}

pub fn close_model_popover(editor_state: &mut EditorState, dirty_mask: &mut u32) {
    editor_state.model_popover.open = false;
    update_model_popover(editor_state, dirty_mask);
}

pub fn handle_model_popover_click(editor_state: &mut EditorState, dirty_mask: &mut u32) {
    let Some(element) = editor_state
        .model_popover
        .element_at(editor_state.input.pointer.local)
    else {
        return;
    };
    if crate::ui::model_popover::setup::activate(
        &mut editor_state.model_popover,
        &mut editor_state.ocr.models,
        element,
    ) {
        if editor_state.model_popover.setup.message.is_empty()
            && matches!(
                element,
                ModelPopoverElement::SystemEngine | ModelPopoverElement::PaddleEngine
            )
        {
            crate::tools::ocr::resources_changed(editor_state, dirty_mask);
        }
        update_model_popover(editor_state, dirty_mask);
        return;
    }
    let (idx, on_button) = match element {
        ModelPopoverElement::Row(idx) => (idx, false),
        ModelPopoverElement::Button(idx) => (idx, true),
        _ => return,
    };
    let is_active = editor_state.ocr.models.active() == Some(idx);

    match editor_state.ocr.models.status(idx) {
        ModelStatus::Installed if !on_button => {
            crate::tools::ocr::use_model(editor_state, idx, dirty_mask)
        }
        ModelStatus::Installed if !is_active => editor_state.ocr.models.remove(idx),
        ModelStatus::Missing | ModelStatus::Failed => {
            editor_state.toasts.dismiss(ToastKind::OcrDownloadFailed);
            editor_state.ocr.models.download(idx);
        }
        ModelStatus::Queued | ModelStatus::Downloading(_) if on_button => {
            crate::tools::ocr::cancel_model_download(editor_state, idx)
        }
        _ => {}
    }

    update_settings_panel(editor_state, dirty_mask);
    update_model_popover(editor_state, dirty_mask);
}

/// once per iteration takes the download progress and updates everything that use it
pub fn tick_model_downloads(editor_state: &mut EditorState, dirty_mask: &mut u32) {
    let was_ready = editor_state.ocr.models.library.status == ModelStatus::Installed;
    let events = editor_state.ocr.models.poll();
    if events.is_empty() {
        return;
    }

    for event in events {
        match event {
            ModelEvent::Progress => {}
            ModelEvent::Ready(idx) => crate::tools::ocr::model_ready(editor_state, idx, dirty_mask),
            ModelEvent::Failed(idx, e) => crate::tools::ocr::model_failed(editor_state, idx, &e),
        }
    }
    if !was_ready
        && editor_state.ocr.models.library.status == ModelStatus::Installed
        && crate::ocr::engine_choice::current() == crate::ocr::engine_choice::Engine::Paddle
    {
        crate::tools::ocr::resources_changed(editor_state, dirty_mask);
    }

    update_settings_panel(editor_state, dirty_mask);
    if editor_state.model_popover.is_visible() {
        update_model_popover(editor_state, dirty_mask);
    }
    apply_damage_rects(editor_state, dirty_mask);
}
