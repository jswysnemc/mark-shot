//! 【线条】【样式交互】共用头部和线型选择的输入、位置、滚动及生命周期

use crate::editor::{DamageZone, EditorState, line_edits};
use crate::features::{arrow::ArrowStyle, line::LineStyle};
use crate::keys::Chord;
use crate::theme::size;
use crate::ui::choice_popover::{ChoiceEvent, ChoicePopover};
use crate::ui::line_properties::LineChoice;
use crate::ui::panel::{UiPanel, emit_panel_damage, sync_panel_hover, sync_panel_rect};
use crate::ui::settings_panel::{SettingsAction, SettingsWidget};

/// 【线条】【面板读取】返回 state 中 kind 对应的只读选择面板
pub fn popup(state: &EditorState, kind: LineChoice) -> &ChoicePopover {
    match kind {
        LineChoice::Heads => &state.arrow_popover,
        LineChoice::Pattern => &state.line_popover,
    }
}

/// 【线条】【面板修改】返回 state 中 kind 对应的可修改选择面板
fn popup_mut(state: &mut EditorState, kind: LineChoice) -> &mut ChoicePopover {
    match kind {
        LineChoice::Heads => &mut state.arrow_popover,
        LineChoice::Pattern => &mut state.line_popover,
    }
}

/// 【线条】【损坏状态拆分】按 kind 借用 state 的面板和损坏列表；返回互不重叠的可修改字段
fn parts(state: &mut EditorState, kind: LineChoice) -> (&mut ChoicePopover, &mut Vec<DamageZone>) {
    match kind {
        LineChoice::Heads => (&mut state.arrow_popover, &mut state.damage_rects),
        LineChoice::Pattern => (&mut state.line_popover, &mut state.damage_rects),
    }
}

/// 【线条】【动作对应】返回 kind 对应的统一属性按钮动作
fn action(kind: LineChoice) -> SettingsAction {
    match kind {
        LineChoice::Heads => SettingsAction::ArrowStyles,
        LineChoice::Pattern => SettingsAction::LineStyles,
    }
}

/// 【线条】【面板可用】判断 state 当前对象是否拥有 kind 对应属性；返回布尔值
fn available(state: &EditorState, kind: LineChoice) -> bool {
    match kind {
        LineChoice::Heads => line_edits::arrow_available(state),
        LineChoice::Pattern => line_edits::available(state),
    }
}

/// 【线条】【面板同步】同步 state 的头部和线型面板，按显示器边界布局；dirty_mask 返回损坏区域，无返回值
pub fn update(state: &mut EditorState, dirty_mask: &mut u32) {
    for kind in LineChoice::ALL {
        update_one(state, kind, dirty_mask);
    }
    sync_buttons(state, dirty_mask);
}

/// 【线条】【单面板同步】按 kind 更新 state 的位置、尺寸和实际选项；dirty_mask 返回重绘范围，无返回值
fn update_one(state: &mut EditorState, kind: LineChoice, dirty_mask: &mut u32) {
    let old = popup(state, kind).rect();
    let monitor = popup(state, kind).monitor_idx;
    if popup(state, kind).open && (!available(state, kind) || state.ui_hidden) {
        popup_mut(state, kind).open = false;
    }
    let mut changed = popup(state, kind).dirty;
    if popup(state, kind).is_visible() {
        let placement = &state.placements[state.settings_panel.monitor_idx];
        let margin = size::margin();
        let max_size = (
            placement.size.0 as f32 - margin * 2.0,
            placement.size.1 as f32 - margin * 2.0,
        );
        changed |= popup_mut(state, kind).resize(max_size);
        let (position, new_monitor) = super::settings::compute_popover_placement(
            state,
            popup(state, kind).layout.size,
            margin,
        );
        let active = Some(match kind {
            LineChoice::Heads => line_edits::current_head(state) as usize,
            LineChoice::Pattern => line_edits::current_style(state) as usize,
        });
        let popup = popup_mut(state, kind);
        popup.position = position;
        popup.render_pos = position;
        popup.monitor_idx = new_monitor;
        if popup.active != active {
            popup.active = active;
            changed = true;
        }
    }
    let (popup, damage) = parts(state, kind);
    sync_panel_rect(popup, old, monitor, changed, damage, dirty_mask);
}

/// 【线条】【面板切换】在 state 中切换 kind，关闭其他属性弹层；dirty_mask 返回重绘范围，无返回值
pub fn toggle(state: &mut EditorState, kind: LineChoice, dirty_mask: &mut u32) {
    super::text::font_popup::close(state, dirty_mask);
    if !available(state, kind) {
        return;
    }
    super::color::close_color_popover(state, dirty_mask);
    super::model::close_model_popover(state, dirty_mask);
    super::marker::close(state, dirty_mask);
    super::ruler::close(state, dirty_mask);
    super::number::close(state, dirty_mask);
    super::filter::close(state, dirty_mask);
    for other in LineChoice::ALL {
        if other != kind {
            close_one(state, other, dirty_mask);
        }
    }
    let panel = popup_mut(state, kind);
    panel.open = !panel.open;
    panel.hovered = None;
    panel.cancel_scroll();
    panel.dirty = true;
    update(state, dirty_mask);
    if let Some(index) = popup(state, kind).active {
        popup_mut(state, kind).ensure_visible(index);
    }
}

/// 【线条】【面板关闭】关闭 state 的 kind 面板并保留淡出动画；dirty_mask 返回重绘范围，无返回值
pub fn close_one(state: &mut EditorState, kind: LineChoice, dirty_mask: &mut u32) {
    if !popup(state, kind).is_visible() {
        return;
    }
    let popup = popup_mut(state, kind);
    popup.open = false;
    popup.hovered = None;
    popup.cancel_scroll();
    popup.dirty = true;
    update(state, dirty_mask);
}

/// 【线条】【全部关闭】关闭 state 的全部线条属性弹层；dirty_mask 返回重绘范围，无返回值
pub fn close(state: &mut EditorState, dirty_mask: &mut u32) {
    for kind in LineChoice::ALL {
        close_one(state, kind, dirty_mask);
    }
}

/// 【线条】【按钮命中】判断 state 的实际指针是否位于 kind 切换按钮；返回布尔值，防止关闭后同次点击重开
pub fn toggle_button_hit(state: &EditorState, kind: LineChoice) -> bool {
    state.input.pointer.monitor_idx == state.settings_panel.monitor_idx
        && matches!(
            state.settings_panel.hit_test(state.input.pointer.local).1.and_then(|index| state.settings_panel.widgets.get(index)),
            Some(SettingsWidget::Action { action: value, .. }) if *value == action(kind)
        )
}

/// 【线条】【样式确认】在 state 中使用 kind、index 对应样式；dirty_mask 返回新旧正文和面板范围，无返回值
fn choose(state: &mut EditorState, kind: LineChoice, index: usize, dirty_mask: &mut u32) {
    match kind {
        LineChoice::Heads => {
            if let Some(style) = ArrowStyle::ALL.get(index) {
                line_edits::set_head(state, *style, dirty_mask);
            }
        }
        LineChoice::Pattern => {
            if let Some(style) = LineStyle::ALL.get(index) {
                line_edits::set_style(state, *style, dirty_mask);
            }
        }
    }
    super::settings::update_settings_panel(state, dirty_mask);
    close_one(state, kind, dirty_mask);
}

/// 【线条】【面板点击】在 state 的 kind 面板内按实际指针选择条目；dirty_mask 返回损坏区域，无返回值
pub fn click(state: &mut EditorState, kind: LineChoice, dirty_mask: &mut u32) {
    if state.input.pointer.monitor_idx == popup(state, kind).monitor_idx
        && let Some(index) = popup(state, kind).item_at(state.input.pointer.local)
    {
        choose(state, kind, index, dirty_mask);
    }
}

/// 【线条】【面板悬停】更新 state 可见样式面板的指针预选；dirty_mask 返回局部损坏区域，无返回值
pub fn pointer_move(state: &mut EditorState, dirty_mask: &mut u32) {
    update(state, dirty_mask);
    for kind in LineChoice::ALL {
        if popup(state, kind).open {
            let hovered = (state.input.pointer.monitor_idx == popup(state, kind).monitor_idx)
                .then(|| popup(state, kind).item_at(state.input.pointer.local))
                .flatten();
            let (popup, damage) = parts(state, kind);
            sync_panel_hover(popup, hovered, damage, dirty_mask);
        }
    }
}

/// 【线条】【面板键盘】在 state 中处理 chord，确认和关闭不触发截图导出；dirty_mask 返回范围，返回是否消费输入
pub fn handle_key(state: &mut EditorState, chord: Chord, dirty_mask: &mut u32) -> bool {
    for kind in LineChoice::ALL {
        if !popup(state, kind).open {
            continue;
        }
        popup_mut(state, kind).cancel_scroll();
        if chord.mods.ctrl || chord.mods.alt || chord.mods.logo {
            close_one(state, kind, dirty_mask);
            return false;
        }
        match popup_mut(state, kind).key(chord.key, chord.mods.shift) {
            Some(ChoiceEvent::Selected(index)) => choose(state, kind, index, dirty_mask),
            Some(ChoiceEvent::Close) => close_one(state, kind, dirty_mask),
            Some(ChoiceEvent::Changed | ChoiceEvent::Unchanged) => update(state, dirty_mask),
            None => {
                close_one(state, kind, dirty_mask);
                return false;
            }
        }
        return true;
    }
    false
}

/// 【线条】【面板滚动】将 delta 用于 state 打开的网格；dirty_mask 返回范围，返回是否阻止底层笔宽滚动
pub fn handle_scroll(state: &mut EditorState, delta: f32, dirty_mask: &mut u32) -> bool {
    for kind in LineChoice::ALL {
        if popup(state, kind).open {
            if state.input.pointer.monitor_idx == popup(state, kind).monitor_idx
                && popup(state, kind).hit_test(state.input.pointer.local)
            {
                popup_mut(state, kind).scroll(-delta);
                pointer_move(state, dirty_mask);
            }
            return true;
        }
    }
    false
}

/// 【线条】【按钮同步】同步 state 中两类属性按钮的打开状态；dirty_mask 返回属性栏范围，无返回值
fn sync_buttons(state: &mut EditorState, dirty_mask: &mut u32) {
    let previous = state.settings_panel.selected;
    let opened = LineChoice::ALL
        .into_iter()
        .find(|kind| popup(state, *kind).open);
    let selected = if let Some(kind) = opened {
        state.settings_panel.widgets.iter().position(|widget| matches!(widget, SettingsWidget::Action { action: value, .. } if *value == action(kind)))
    } else {
        previous.filter(|index| {
            !matches!(
                state.settings_panel.widgets.get(*index),
                Some(SettingsWidget::Action {
                    action: SettingsAction::ArrowStyles | SettingsAction::LineStyles,
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
