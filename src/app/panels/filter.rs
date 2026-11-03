//! 【滤镜】【选择交互】管理五模式弹层的输入和生命周期，复用统一布局、命中和动画

use crate::editor::{EditorState, filter_edits};
use crate::features::filter::FilterMode;
use crate::keys::Chord;
use crate::theme::size;
use crate::ui::choice_popover::ChoiceEvent;
use crate::ui::panel::{UiPanel, emit_panel_damage, sync_panel_hover, sync_panel_rect};
use crate::ui::settings_panel::{SettingsAction, SettingsWidget};

/// 【滤镜】【弹层同步】根据 state 当前对象同步弹层位置和模式；dirty_mask 返回损坏区域，无返回值
pub fn update(state: &mut EditorState, dirty_mask: &mut u32) {
    let old = state.filter_popover.rect();
    let old_monitor = state.filter_popover.monitor_idx;
    if state.filter_popover.open
        && (!filter_edits::available(state) || state.ui_hidden || state.tool_active)
    {
        state.filter_popover.open = false;
        state.filter_popover.cancel_scroll();
    }
    let mut changed = state.filter_popover.dirty;
    if state.filter_popover.is_visible() {
        let placement = &state.placements[state.settings_panel.monitor_idx];
        let margin = size::margin();
        changed |= state.filter_popover.resize((
            placement.size.0 as f32 - margin * 2.0,
            placement.size.1 as f32 - margin * 2.0,
        ));
        let (position, monitor) = super::settings::compute_popover_placement(
            state,
            state.filter_popover.layout.size,
            margin,
        );
        let active = Some(filter_edits::current_mode(state) as usize);
        state.filter_popover.position = position;
        state.filter_popover.render_pos = position;
        state.filter_popover.monitor_idx = monitor;
        if state.filter_popover.active != active {
            state.filter_popover.active = active;
            changed = true;
        }
    }
    sync_panel_rect(
        &mut state.filter_popover,
        old,
        old_monitor,
        changed,
        &mut state.damage_rects,
        dirty_mask,
    );
    sync_button(state, dirty_mask);
}

/// 【滤镜】【弹层切换】在 state 中切换模式面板并关闭其他弹层；dirty_mask 返回范围，无返回值
pub fn toggle(state: &mut EditorState, dirty_mask: &mut u32) {
    super::text::font_popup::close(state, dirty_mask);
    if !filter_edits::available(state) {
        return;
    }
    super::color::close_color_popover(state, dirty_mask);
    super::model::close_model_popover(state, dirty_mask);
    super::marker::close(state, dirty_mask);
    super::ruler::close(state, dirty_mask);
    super::number::close(state, dirty_mask);
    super::line::close(state, dirty_mask);
    state.filter_popover.open = !state.filter_popover.open;
    state.filter_popover.hovered = None;
    state.filter_popover.cancel_scroll();
    state.filter_popover.dirty = true;
    update(state, dirty_mask);
    if let Some(index) = state.filter_popover.active {
        state.filter_popover.ensure_visible(index);
    }
}

/// 【滤镜】【弹层关闭】关闭 state 的模式面板，保留统一淡出动画；dirty_mask 返回范围，无返回值
pub fn close(state: &mut EditorState, dirty_mask: &mut u32) {
    if !state.filter_popover.is_visible() {
        return;
    }
    state.filter_popover.open = false;
    state.filter_popover.hovered = None;
    state.filter_popover.cancel_scroll();
    state.filter_popover.dirty = true;
    update(state, dirty_mask);
}

/// 【滤镜】【切换按钮命中】返回 state 的实际指针是否在当前模式按钮，阻止同次点击关闭后重开
pub fn toggle_button_hit(state: &EditorState) -> bool {
    state.input.pointer.monitor_idx == state.settings_panel.monitor_idx
        && matches!(
            state
                .settings_panel
                .hit_test(state.input.pointer.local)
                .1
                .and_then(|index| state.settings_panel.widgets.get(index)),
            Some(SettingsWidget::Action {
                action: SettingsAction::FilterModes,
                ..
            })
        )
}

/// 【滤镜】【模式确认】把 index 对应模式用于 state 当前对象；dirty_mask 返回新旧图像与面板范围，无返回值
fn choose(state: &mut EditorState, index: usize, dirty_mask: &mut u32) {
    if let Some(mode) = FilterMode::ALL.get(index) {
        filter_edits::set_mode(state, *mode, dirty_mask);
    }
    super::settings::update_settings_panel(state, dirty_mask);
    close(state, dirty_mask);
}

/// 【滤镜】【面板点击】根据 state 实际指针选择模式；dirty_mask 返回范围，空白处不修改强度，无返回值
pub fn click(state: &mut EditorState, dirty_mask: &mut u32) {
    if state.input.pointer.monitor_idx == state.filter_popover.monitor_idx
        && let Some(index) = state.filter_popover.item_at(state.input.pointer.local)
    {
        choose(state, index, dirty_mask);
    }
}

/// 【滤镜】【面板悬停】同步 state 模式面板的位置和指针预选；dirty_mask 返回局部损坏范围，无返回值
pub fn pointer_move(state: &mut EditorState, dirty_mask: &mut u32) {
    update(state, dirty_mask);
    if state.filter_popover.open {
        let hovered = (state.input.pointer.monitor_idx == state.filter_popover.monitor_idx)
            .then(|| state.filter_popover.item_at(state.input.pointer.local))
            .flatten();
        sync_panel_hover(
            &mut state.filter_popover,
            hovered,
            &mut state.damage_rects,
            dirty_mask,
        );
    }
}

/// 【滤镜】【面板键盘】在 state 中处理 chord；dirty_mask 返回范围，返回是否消费，确认和 Escape 不导出或关闭截图
pub fn handle_key(state: &mut EditorState, chord: Chord, dirty_mask: &mut u32) -> bool {
    if !state.filter_popover.open {
        return false;
    }
    state.filter_popover.cancel_scroll();
    if chord.mods.ctrl || chord.mods.alt || chord.mods.logo {
        close(state, dirty_mask);
        return false;
    }
    match state.filter_popover.key(chord.key, chord.mods.shift) {
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

/// 【滤镜】【面板滚动】将 delta 用于 state 打开的模式网格；dirty_mask 返回范围，返回是否阻止底层强度滚动
pub fn handle_scroll(state: &mut EditorState, delta: f32, dirty_mask: &mut u32) -> bool {
    if !state.filter_popover.open {
        return false;
    }
    if state.input.pointer.monitor_idx == state.filter_popover.monitor_idx
        && state.filter_popover.hit_test(state.input.pointer.local)
    {
        state.filter_popover.scroll(-delta);
        pointer_move(state, dirty_mask);
    }
    true
}

/// 【滤镜】【按钮状态】把 state 模式按钮同步为面板打开状态；dirty_mask 返回属性栏范围，无返回值
fn sync_button(state: &mut EditorState, dirty_mask: &mut u32) {
    let previous = state.settings_panel.selected;
    let selected = if state.filter_popover.open {
        state.settings_panel.widgets.iter().position(|widget| {
            matches!(
                widget,
                SettingsWidget::Action {
                    action: SettingsAction::FilterModes,
                    ..
                }
            )
        })
    } else {
        previous.filter(|index| {
            !matches!(
                state.settings_panel.widgets.get(*index),
                Some(SettingsWidget::Action {
                    action: SettingsAction::FilterModes,
                    ..
                })
            )
        })
    };
    if selected != previous {
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
}
