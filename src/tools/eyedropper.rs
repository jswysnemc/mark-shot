// Color picker tool: inspects raw pixels directly from base (original) screenshot layer
//
// Displays live color values in the magnifier (`ui::magnifier`) while locking the last
// selected color in the options panel. This prevents the value from drifting as the mouse
// moves toward the UI. Clicking the panel copies the value.
//
// Used in two ways via `pick`:
// 1. Toolbar tool (continuously picks colors while active and copies it to the clipboard).
// 2. Palette eye-dropper button (picks a single color without switching tools: `EditorState::pick_once`).

use tiny_skia::Color;

use crate::editor::{DamageZone, EditorState};
use crate::tools::ToolBehavior;
use crate::types::{MouseButton, SpecialKey};
use crate::ui::magnifier::{magnifier_rect, sample_pixel};
use crate::ui::settings_panel::ValueField;
use crate::ui::toast::ToastKind;

pub struct EyedropperTool;

impl ToolBehavior for EyedropperTool {
    fn on_button(
        &self,
        state: &mut EditorState,
        button: MouseButton,
        pressed: bool,
        dirty_mask: &mut u32,
    ) {
        if pressed && matches!(button, MouseButton::Left) {
            pick(state, dirty_mask);
        }
    }

    fn on_move(&self, _state: &mut EditorState, _global: (f64, f64), _dirty_mask: &mut u32) {}

    fn on_key(&self, state: &mut EditorState, key: SpecialKey, _dirty_mask: &mut u32) {
        if state.input.ctrl && matches!(key, SpecialKey::KeyC) {
            let color = state.tool_settings.color;
            copy_value(state, ValueField::Hex, color);
        }
    }

    fn on_activate(&self, state: &mut EditorState, dirty_mask: &mut u32) {
        damage_loupe(state, dirty_mask);
    }

    fn on_deactivate(&self, state: &mut EditorState, dirty_mask: &mut u32) {
        damage_loupe(state, dirty_mask);
    }
}

pub fn pick(state: &mut EditorState, dirty_mask: &mut u32) {
    let Some(color) = color_under_pointer(state) else {
        return;
    };

    crate::editor::edits::use_color(state, color, dirty_mask);
    copy_value(state, ValueField::Hex, color);

    if state.pick_once {
        end_pick_once(state, dirty_mask);
    }
}

pub fn toggle_pick_once(state: &mut EditorState, dirty_mask: &mut u32) {
    if state.pick_once {
        end_pick_once(state, dirty_mask);
        return;
    }
    state.pick_once = true;
    state
        .toasts
        .show(ToastKind::PickColor, &mut state.text.font_system);
    damage_loupe(state, dirty_mask);
}

pub fn end_pick_once(state: &mut EditorState, dirty_mask: &mut u32) {
    if !state.pick_once {
        return;
    }
    state.pick_once = false;
    state.toasts.dismiss(ToastKind::PickColor);
    damage_loupe(state, dirty_mask);
}

pub fn copy_value(state: &mut EditorState, field: ValueField, color: Color) {
    let text = field.text(color);
    let message = match crate::utils::copy_to_clipboard(&text) {
        Ok(()) => format!("Copied {text}"),
        Err(e) => {
            log::info!("eyedropper: can't copy: {e}");
            format!("Couldn't copy: {e}")
        }
    };
    state
        .toasts
        .show_text(ToastKind::ColorCopied, message, &mut state.text.font_system);
}

fn color_under_pointer(state: &EditorState) -> Option<Color> {
    let capture = state.captures.get(state.input.pointer.monitor_idx)?;
    sample_pixel(
        &capture.pixmap,
        capture.to_native(state.input.pointer.local),
    )
}

fn damage_loupe(state: &mut EditorState, dirty_mask: &mut u32) {
    let monitor_idx = state.input.pointer.monitor_idx;
    let Some(placement) = state.placements.get(monitor_idx) else {
        return;
    };
    let (mw, mh) = (placement.size.0 as f32, placement.size.1 as f32);
    let cursor = (
        state.input.pointer.local.0 as f32,
        state.input.pointer.local.1 as f32,
    );
    let rect = magnifier_rect(cursor, mw, mh, true);
    state
        .damage_rects
        .push(DamageZone::Local { monitor_idx, rect });
    crate::editor::dirty::mark_dirty(dirty_mask, monitor_idx);
}
