//! 【序号】【样式面板】复用统一网格、键盘和滚动，修改当前序号或后续默认样式

use crate::editor::EditorState;
use crate::editor::number_edits::{available, current_style, set_style};
use crate::features::number::NumberStyle;
use crate::keys::Chord;
use crate::theme::size;
use crate::ui::choice_popover::ChoiceEvent;
use crate::ui::panel::{UiPanel, emit_panel_damage, sync_panel_hover, sync_panel_rect};
use crate::ui::settings_panel::{SettingsAction, SettingsWidget};

/// 【序号】【样式面板同步】按 state 适配显示器空间、位置和选中样式；dirty_mask 返回面板重绘范围，无返回值
pub fn update(state: &mut EditorState, dirty_mask: &mut u32) {
    let old = state.number_popover.rect();
    let monitor = state.number_popover.monitor_idx;
    if state.number_popover.open && (!available(state) || state.ui_hidden) {
        state.number_popover.open = false;
    }
    let mut changed = state.number_popover.dirty;
    if state.number_popover.is_visible() {
        let placement = &state.placements[state.settings_panel.monitor_idx];
        let margin = size::margin();
        changed |= state.number_popover.resize((
            placement.size.0 as f32 - margin * 2.0,
            placement.size.1 as f32 - margin * 2.0,
        ));
        let (position, monitor) = super::settings::compute_popover_placement(
            state,
            state.number_popover.layout.size,
            margin,
        );
        state.number_popover.position = position;
        state.number_popover.render_pos = position;
        state.number_popover.monitor_idx = monitor;
        let active = NumberStyle::ALL
            .iter()
            .position(|style| *style == current_style(state));
        if state.number_popover.active != active {
            state.number_popover.active = active;
            changed = true;
        }
    }
    sync_panel_rect(
        &mut state.number_popover,
        old,
        monitor,
        changed,
        &mut state.damage_rects,
        dirty_mask,
    );
    sync_button(state, dirty_mask);
}

/// 【序号】【样式面板切换】在 state 中切换样式弹层并关闭其他弹层；dirty_mask 返回重绘范围，无返回值
pub fn toggle(state: &mut EditorState, dirty_mask: &mut u32) {
    super::text::font_popup::close(state, dirty_mask);
    if !available(state) {
        return;
    }
    super::color::close_color_popover(state, dirty_mask);
    super::model::close_model_popover(state, dirty_mask);
    super::marker::close(state, dirty_mask);
    super::ruler::close(state, dirty_mask);
    super::line::close(state, dirty_mask);
    super::filter::close(state, dirty_mask);
    state.number_popover.open = !state.number_popover.open;
    state.number_popover.hovered = None;
    state.number_popover.cancel_scroll();
    state.number_popover.dirty = true;
    update(state, dirty_mask);
    if let Some(index) = state.number_popover.active {
        state.number_popover.ensure_visible(index);
    }
}

/// 【序号】【样式面板关闭】关闭 state 的弹层，保留淡出动画；dirty_mask 返回重绘范围，无返回值
pub fn close(state: &mut EditorState, dirty_mask: &mut u32) {
    if !state.number_popover.is_visible() {
        return;
    }
    state.number_popover.open = false;
    state.number_popover.hovered = None;
    state.number_popover.cancel_scroll();
    state.number_popover.dirty = true;
    update(state, dirty_mask);
}

/// 【序号】【样式按钮命中】返回 state 的实际指针是否位于样式按钮，避免关闭后同一次点击重新打开
pub fn toggle_button_hit(state: &EditorState) -> bool {
    state.input.pointer.monitor_idx == state.settings_panel.monitor_idx
        && matches!(
            state
                .settings_panel
                .hit_test(state.input.pointer.local)
                .1
                .and_then(|index| state.settings_panel.widgets.get(index)),
            Some(SettingsWidget::Action {
                action: SettingsAction::NumberStyles,
                ..
            })
        )
}

/// 【序号】【样式选择】把 index 对应样式用于 state；已有序号形成单次撤销，dirty_mask 返回属性更新，无返回值
fn choose(state: &mut EditorState, index: usize, dirty_mask: &mut u32) {
    let Some(&style) = NumberStyle::ALL.get(index) else {
        return;
    };
    set_style(state, style, dirty_mask);
    super::settings::update_settings_panel(state, dirty_mask);
    close(state, dirty_mask);
}

/// 【序号】【样式点击】使用 state 的实际指针命中样式条目；dirty_mask 返回重绘范围，无返回值
pub fn click(state: &mut EditorState, dirty_mask: &mut u32) {
    if state.input.pointer.monitor_idx == state.number_popover.monitor_idx
        && let Some(index) = state.number_popover.item_at(state.input.pointer.local)
    {
        choose(state, index, dirty_mask);
    }
}

/// 【序号】【样式悬停】用 state 的实际指针更新预选，不覆盖未移动指针时的键盘预选；dirty_mask 返回重绘范围
pub fn pointer_move(state: &mut EditorState, dirty_mask: &mut u32) {
    update(state, dirty_mask);
    if state.number_popover.open {
        let hovered = (state.input.pointer.monitor_idx == state.number_popover.monitor_idx)
            .then(|| state.number_popover.item_at(state.input.pointer.local))
            .flatten();
        sync_panel_hover(
            &mut state.number_popover,
            hovered,
            &mut state.damage_rects,
            dirty_mask,
        );
    }
}

/// 【序号】【样式键盘】处理 state 的 chord，确认和关闭不传给截图快捷键；dirty_mask 返回重绘范围，返回是否消费输入
pub fn handle_key(state: &mut EditorState, chord: Chord, dirty_mask: &mut u32) -> bool {
    if !state.number_popover.open {
        return false;
    }
    state.number_popover.cancel_scroll();
    if chord.mods.ctrl || chord.mods.alt || chord.mods.logo {
        close(state, dirty_mask);
        return false;
    }
    match state.number_popover.key(chord.key, chord.mods.shift) {
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

/// 【序号】【样式滚动】把 delta 应用于 state 打开的网格；dirty_mask 返回重绘范围，返回是否阻止笔宽滚动
pub fn handle_scroll(state: &mut EditorState, delta: f32, dirty_mask: &mut u32) -> bool {
    if !state.number_popover.open {
        return false;
    }
    if state.input.pointer.monitor_idx == state.number_popover.monitor_idx
        && state.number_popover.hit_test(state.input.pointer.local)
    {
        state.number_popover.scroll(-delta);
        pointer_move(state, dirty_mask);
    }
    true
}

/// 【序号】【样式按钮状态】同步 state 中样式按钮的打开状态；dirty_mask 返回属性栏重绘范围，无返回值
fn sync_button(state: &mut EditorState, dirty_mask: &mut u32) {
    let Some(index) = state.settings_panel.widgets.iter().position(|widget| {
        matches!(
            widget,
            SettingsWidget::Action {
                action: SettingsAction::NumberStyles,
                ..
            }
        )
    }) else {
        return;
    };
    let selected = if state.number_popover.open {
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
