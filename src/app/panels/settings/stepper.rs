//! 【属性面板】【步进编辑】管理数值字段、滚轮累积与按住重复输入

use super::update_settings_panel;
use super::values::{apply_stepper_field, stepper_value};
use crate::editor::EditorState;
use crate::editor::dirty::apply_damage_rects;
use crate::types::SpecialKey;
use crate::ui::panel::{UiPanel, emit_panel_damage};
use crate::ui::settings_panel::{SettingsWidget, StepperArrow, StepperRange};
use std::time::{Duration, Instant};

/// 【属性面板】【步进编辑】推进 editor_state 的按住重复时钟；dirty_mask 返回重绘显示器，无返回值
pub fn tick_stepper_arrow_hold(editor_state: &mut EditorState, dirty_mask: &mut u32) {
    let Some(hold) = editor_state.settings_panel.arrow_held else {
        return;
    };
    let now = Instant::now();
    let input = &crate::config::get().input;

    let next_due = if hold.repeat_count == 0 {
        hold.started_at + Duration::from_millis(input.hold_delay_ms)
    } else {
        let interval = if hold.repeat_count >= input.hold_fast_after {
            input.hold_fast_repeat_ms
        } else {
            input.hold_repeat_ms
        };
        hold.last_step_at + Duration::from_millis(interval)
    };

    if now < next_due {
        return;
    }

    apply_stepper_arrow_step(editor_state, hold.widget_idx, hold.arrow, dirty_mask);
    update_settings_panel(editor_state, dirty_mask);

    if let Some(hold) = editor_state.settings_panel.arrow_held.as_mut() {
        hold.repeat_count += 1;
        hold.last_step_at = now;
    }
}

/// 【属性面板】【步进编辑】把 ch 插入 editor_state 的数值字段并实时应用；dirty_mask 返回重绘显示器，无返回值
pub fn handle_settings_text_input(editor_state: &mut EditorState, ch: char, dirty_mask: &mut u32) {
    let monitor_idx = editor_state.settings_panel.monitor_idx;
    let Some(rect) = editor_state.settings_panel.rect() else {
        return;
    };

    if editor_state.settings_panel.insert_char(ch) {
        emit_panel_damage(
            rect,
            monitor_idx,
            &mut editor_state.damage_rects,
            dirty_mask,
        );

        if let Some(widget_idx) = editor_state
            .settings_panel
            .fields
            .editing
            .as_ref()
            .map(|e| e.key)
        {
            live_apply_stepper_field(editor_state, widget_idx, dirty_mask);
        }
    }
}

/// 【属性面板】【步进编辑】处理 editor_state 数值字段的 key；dirty_mask 返回重绘显示器，无返回值
pub fn handle_settings_key_press(
    editor_state: &mut EditorState,
    key: SpecialKey,
    dirty_mask: &mut u32,
) {
    let monitor_idx = editor_state.settings_panel.monitor_idx;
    let Some(rect) = editor_state.settings_panel.rect() else {
        return;
    };

    let ctrl = editor_state.input.ctrl;
    let shift = editor_state.input.shift;

    let (changed, commit) = editor_state.settings_panel.handle_key(key, ctrl, shift);

    if changed {
        emit_panel_damage(
            rect,
            monitor_idx,
            &mut editor_state.damage_rects,
            dirty_mask,
        );

        if let Some(widget_idx) = editor_state
            .settings_panel
            .fields
            .editing
            .as_ref()
            .map(|e| e.key)
        {
            live_apply_stepper_field(editor_state, widget_idx, dirty_mask);
        }
    }

    if commit {
        commit_stepper_text_edit(editor_state, dirty_mask);
    }
}

/// 【属性面板】【步进编辑】确认 editor_state 当前字段并形成一次历史记录；dirty_mask 返回重绘显示器，无返回值
pub fn commit_stepper_text_edit(editor_state: &mut EditorState, dirty_mask: &mut u32) {
    let monitor_idx = editor_state.settings_panel.monitor_idx;
    let rect = editor_state.settings_panel.rect();

    let snapshot = editor_state.settings_panel.pre_edit_snapshot.take();

    let Some((widget_idx, text)) = editor_state.settings_panel.commit_edit() else {
        return;
    };

    if let Some(rect) = rect {
        emit_panel_damage(
            rect,
            monitor_idx,
            &mut editor_state.damage_rects,
            dirty_mask,
        );
    }

    try_apply_stepper_text(editor_state, widget_idx, &text, false, dirty_mask);

    update_settings_panel(editor_state, dirty_mask);
    apply_damage_rects(editor_state, dirty_mask);

    editor_state.commit_snapshot(snapshot);
}

/// 【属性面板】【步进编辑】解析 widget_idx 的 text 并写入 editor_state；record_undo 控制历史，dirty_mask 返回重绘显示器，返回解析是否成功
fn try_apply_stepper_text(
    editor_state: &mut EditorState,
    widget_idx: usize,
    text: &str,
    record_undo: bool,
    dirty_mask: &mut u32,
) -> bool {
    let Some(SettingsWidget::Stepper { range, .. }) =
        editor_state.settings_panel.widgets.get(widget_idx)
    else {
        return false;
    };
    let range = *range;
    let (min, max, _) = range.get();

    let Ok(parsed) = text.parse::<f32>() else {
        return false;
    };
    if !parsed.is_finite() {
        return false;
    }
    apply_stepper_field(
        editor_state,
        range,
        parsed.clamp(min, max),
        record_undo,
        dirty_mask,
    );
    true
}

/// 【属性面板】【步进编辑】实时应用 editor_state 中 widget_idx 的编辑内容；dirty_mask 返回重绘显示器，无返回值
pub(crate) fn live_apply_stepper_field(
    editor_state: &mut EditorState,
    widget_idx: usize,
    dirty_mask: &mut u32,
) {
    let Some(text) = editor_state
        .settings_panel
        .fields
        .editing
        .as_ref()
        .filter(|e| e.key == widget_idx)
        .map(|e| e.field.text.clone())
    else {
        return;
    };

    try_apply_stepper_text(editor_state, widget_idx, &text, false, dirty_mask);
}

/// 【属性面板】【步进编辑】按 arrow 调整 editor_state 中 widget_idx 数值；dirty_mask 返回重绘显示器，无返回值
pub fn apply_stepper_arrow_step(
    editor_state: &mut EditorState,
    widget_idx: usize,
    arrow: StepperArrow,
    dirty_mask: &mut u32,
) {
    if editor_state.settings_panel.pre_edit_snapshot.is_none() {
        editor_state.settings_panel.pre_edit_snapshot = Some(editor_state.snapshot());
    }

    let Some(SettingsWidget::Stepper { range, .. }) =
        editor_state.settings_panel.widgets.get(widget_idx)
    else {
        return;
    };
    let range = *range;
    let (min, max, step) = range.get();
    let current = stepper_value(editor_state, range);

    let delta = if arrow == StepperArrow::Up {
        step
    } else {
        -step
    };
    let new_value = (current + delta).clamp(min, max);

    apply_stepper_field(editor_state, range, new_value, false, dirty_mask);
}

/// 【属性面板】【步进编辑】以 editor_state 中 widget_idx 已确认的数值替换编辑缓冲；无返回值
pub fn sync_stepper_edit_text(editor_state: &mut EditorState, widget_idx: usize) {
    if editor_state
        .settings_panel
        .fields
        .editing
        .as_ref()
        .is_none_or(|e| e.key != widget_idx)
    {
        return;
    }

    let Some(SettingsWidget::Stepper { range, .. }) =
        editor_state.settings_panel.widgets.get(widget_idx)
    else {
        return;
    };
    let value = stepper_value(editor_state, *range);

    editor_state
        .settings_panel
        .fields
        .set_editing_text(widget_idx, format_stepper_number(value, *range));
    editor_state.settings_panel.dirty = true;
}

/// 【属性面板】【步进编辑】将 steps 应用到 editor_state 的 widget_idx；dirty_mask 返回重绘显示器，无返回值
pub fn handle_stepper_scroll(
    editor_state: &mut EditorState,
    widget_idx: usize,
    delta_y: f32,
    dirty_mask: &mut u32,
) {
    if !matches!(
        editor_state.settings_panel.widgets.get(widget_idx),
        Some(SettingsWidget::Stepper { .. })
    ) {
        return;
    }

    let steps = editor_state.settings_panel.scroll_step(
        widget_idx,
        delta_y / crate::config::get().input.scroll_step_pixels,
    );

    if steps == 0 {
        return;
    }

    if editor_state.settings_panel.pre_edit_snapshot.is_none() {
        editor_state.settings_panel.pre_edit_snapshot = Some(editor_state.snapshot());
    }

    let arrow = if steps > 0 {
        StepperArrow::Up
    } else {
        StepperArrow::Down
    };
    for _ in 0..steps.unsigned_abs() {
        apply_stepper_arrow_step(editor_state, widget_idx, arrow, dirty_mask);
    }

    update_settings_panel(editor_state, dirty_mask);

    sync_stepper_edit_text(editor_state, widget_idx);
}

/// 【属性面板】【步进编辑】根据 range 将 v 格式化为字段文字；放大倍率固定保留两位小数，其他数值保留现有精度
pub(super) fn format_stepper_number(v: f32, range: StepperRange) -> String {
    if range == StepperRange::Magnification {
        format!("{v:.2}")
    } else if v.fract() == 0.0 {
        format!("{}", v as i32)
    } else {
        format!("{:.1}", v)
    }
}
