//! 【尺子】【模式面板】使用统一选择弹层调整下一次测量模式，不转换已完成尺子

use crate::editor::EditorState;
use crate::editor::edits::active_annotation_idx;
use crate::features::ruler::RulerMode;
use crate::keys::Chord;
use crate::theme::size;
use crate::tools::Tool;
use crate::types::AnnotationShape;
use crate::ui::choice_popover::ChoiceEvent;
use crate::ui::panel::{UiPanel, emit_panel_damage, sync_panel_hover, sync_panel_rect};
use crate::ui::settings_panel::{SettingsAction, SettingsWidget};

/// 【尺子】【面板来源】返回 state 是否使用尺子或选中已完成尺子
fn available(state: &EditorState) -> bool {
    state.selected_tool == Tool::Ruler
        || active_annotation_idx(state)
            .and_then(|index| state.annotations.get(index))
            .is_some_and(|annotation| matches!(annotation.shape, AnnotationShape::Ruler { .. }))
}

/// 【尺子】【模式面板同步】适配 state 的面板位置与当前默认模式；dirty_mask 返回重绘显示器，无返回值
pub fn update(state: &mut EditorState, dirty_mask: &mut u32) {
    let old = state.ruler_popover.rect();
    let monitor = state.ruler_popover.monitor_idx;
    if state.ruler_popover.open && (!available(state) || state.ui_hidden) {
        state.ruler_popover.open = false;
    }
    let mut changed = state.ruler_popover.dirty;
    if state.ruler_popover.is_visible() {
        let placement = &state.placements[state.settings_panel.monitor_idx];
        let margin = size::margin();
        changed |= state.ruler_popover.resize((
            placement.size.0 as f32 - margin * 2.0,
            placement.size.1 as f32 - margin * 2.0,
        ));
        let (position, monitor) = super::settings::compute_popover_placement(
            state,
            state.ruler_popover.layout.size,
            margin,
        );
        state.ruler_popover.position = position;
        state.ruler_popover.render_pos = position;
        state.ruler_popover.monitor_idx = monitor;
        let active = RulerMode::ALL
            .iter()
            .position(|mode| *mode == state.tool_settings.ruler_mode);
        if state.ruler_popover.active != active {
            state.ruler_popover.active = active;
            changed = true;
        }
    }
    sync_panel_rect(
        &mut state.ruler_popover,
        old,
        monitor,
        changed,
        &mut state.damage_rects,
        dirty_mask,
    );
    sync_button(state, dirty_mask);
}

/// 【尺子】【模式面板切换】打开或关闭 state 的模式面板，并关闭其他弹层；dirty_mask 返回重绘显示器，无返回值
pub fn toggle(state: &mut EditorState, dirty_mask: &mut u32) {
    super::text::font_popup::close(state, dirty_mask);
    if !available(state) {
        return;
    }
    super::color::close_color_popover(state, dirty_mask);
    super::model::close_model_popover(state, dirty_mask);
    super::marker::close(state, dirty_mask);
    super::number::close(state, dirty_mask);
    super::line::close(state, dirty_mask);
    super::filter::close(state, dirty_mask);
    state.ruler_popover.open = !state.ruler_popover.open;
    state.ruler_popover.hovered = None;
    state.ruler_popover.cancel_scroll();
    state.ruler_popover.dirty = true;
    update(state, dirty_mask);
}

/// 【尺子】【模式面板关闭】关闭 state 的模式弹层并保留淡出动画；dirty_mask 返回重绘显示器，无返回值
pub fn close(state: &mut EditorState, dirty_mask: &mut u32) {
    if !state.ruler_popover.is_visible() {
        return;
    }
    state.ruler_popover.open = false;
    state.ruler_popover.hovered = None;
    state.ruler_popover.cancel_scroll();
    state.ruler_popover.dirty = true;
    update(state, dirty_mask);
}

/// 【尺子】【模式按钮命中】返回 state 的指针是否位于模式选择按钮
pub fn toggle_button_hit(state: &EditorState) -> bool {
    state.input.pointer.monitor_idx == state.settings_panel.monitor_idx
        && matches!(
            state
                .settings_panel
                .hit_test(state.input.pointer.local)
                .1
                .and_then(|index| state.settings_panel.widgets.get(index)),
            Some(SettingsWidget::Action {
                action: SettingsAction::RulerModes,
                ..
            })
        )
}

/// 【尺子】【模式选择】把 index 对应模式用于 state 的后续测量；dirty_mask 返回重绘显示器，不产生标注历史，无返回值
fn choose(state: &mut EditorState, index: usize, dirty_mask: &mut u32) {
    let Some(&mode) = RulerMode::ALL.get(index) else {
        return;
    };
    crate::editor::ruler_session::set_mode(state, mode, false, dirty_mask);
    super::settings::update_settings_panel(state, dirty_mask);
    close(state, dirty_mask);
}

/// 【尺子】【模式点击】以 state 当前指针命中模式条目；dirty_mask 返回重绘显示器，无返回值
pub fn click(state: &mut EditorState, dirty_mask: &mut u32) {
    if state.input.pointer.monitor_idx == state.ruler_popover.monitor_idx
        && let Some(index) = state.ruler_popover.item_at(state.input.pointer.local)
    {
        choose(state, index, dirty_mask);
    }
}

/// 【尺子】【模式悬停】使用 state 的实际指针更新预选；dirty_mask 返回重绘显示器，不覆盖未移动指针时的键盘预选
pub fn pointer_move(state: &mut EditorState, dirty_mask: &mut u32) {
    update(state, dirty_mask);
    if state.ruler_popover.open {
        let hovered = (state.input.pointer.monitor_idx == state.ruler_popover.monitor_idx)
            .then(|| state.ruler_popover.item_at(state.input.pointer.local))
            .flatten();
        sync_panel_hover(
            &mut state.ruler_popover,
            hovered,
            &mut state.damage_rects,
            dirty_mask,
        );
    }
}

/// 【尺子】【模式键盘】处理 state 的 chord；dirty_mask 返回重绘显示器，返回是否消费面板导航与确认
pub fn handle_key(state: &mut EditorState, chord: Chord, dirty_mask: &mut u32) -> bool {
    if !state.ruler_popover.open {
        return false;
    }
    if chord.mods.ctrl || chord.mods.alt || chord.mods.logo {
        close(state, dirty_mask);
        return false;
    }
    match state.ruler_popover.key(chord.key, chord.mods.shift) {
        Some(ChoiceEvent::Selected(index)) => choose(state, index, dirty_mask),
        Some(ChoiceEvent::Close) => close(state, dirty_mask),
        Some(ChoiceEvent::Changed | ChoiceEvent::Unchanged) => update(state, dirty_mask),
        None => {
            close(state, dirty_mask);
            return false;
        }
    }
    true
}

/// 【尺子】【模式滚动】消费 state 打开面板中的 delta；dirty_mask 返回重绘显示器，不把滚轮传给笔宽字段
pub fn handle_scroll(state: &mut EditorState, delta: f32, dirty_mask: &mut u32) -> bool {
    if !state.ruler_popover.open {
        return false;
    }
    if state.input.pointer.monitor_idx == state.ruler_popover.monitor_idx
        && state.ruler_popover.hit_test(state.input.pointer.local)
    {
        state.ruler_popover.scroll(-delta);
        pointer_move(state, dirty_mask);
    }
    true
}

/// 【尺子】【模式按钮同步】把 state 的模式面板打开状态同步到按钮；dirty_mask 返回属性栏重绘，无返回值
fn sync_button(state: &mut EditorState, dirty_mask: &mut u32) {
    let Some(index) = state.settings_panel.widgets.iter().position(|widget| {
        matches!(
            widget,
            SettingsWidget::Action {
                action: SettingsAction::RulerModes,
                ..
            }
        )
    }) else {
        return;
    };
    let selected = if state.ruler_popover.open {
        Some(index)
    } else {
        state
            .settings_panel
            .selected
            .filter(|selected| *selected != index)
    };
    if selected == state.settings_panel.selected {
        return;
    }
    state.settings_panel.selected = selected;
    state.settings_panel.dirty = true;
    if let Some(rect) = state.settings_panel.rect() {
        emit_panel_damage(
            rect,
            state.settings_panel.monitor_idx,
            &mut state.damage_rects,
            dirty_mask,
        );
    }
}
