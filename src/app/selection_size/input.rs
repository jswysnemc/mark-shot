//! 【选区尺寸】【指针归属】字段、锁和比例按钮消费自身输入，NoFocus 背景与锁保留待提交文字

use crate::{
    editor::EditorState,
    types::{MouseButton, ScrollEvent},
    ui::{
        panel::UiPanel,
        selection_size::state::{Hit, contains},
    },
};

/// 【选区尺寸】【界面命中】按 state 当前指针返回面板目标，避让弹层及位于上层的工具栏，空区域返回空值
pub(in crate::app) fn hit(state: &EditorState) -> Option<Hit> {
    if state.input.mouse_down || state.ann_drag.is_some() || state.toolbar_drag.is_some() {
        return None;
    }
    let panel = state
        .selection_size
        .panel
        .as_ref()
        .filter(|panel| panel.monitor == state.input.pointer.monitor_idx)?;
    let point = state.input.pointer.local;
    let hit = panel.hit(point)?;
    if state
        .font_panel
        .rect()
        .is_some_and(|rect| state.font_panel.monitor_idx == panel.monitor && contains(rect, point))
        || [
            &state.marker_popover,
            &state.ruler_popover,
            &state.number_popover,
            &state.arrow_popover,
            &state.line_popover,
            &state.filter_popover,
        ]
        .iter()
        .any(|popup| {
            popup.monitor_idx == panel.monitor
                && popup.rect().is_some_and(|rect| contains(rect, point))
        })
        || state.color_popover.rect().is_some_and(|rect| {
            state.color_popover.monitor_idx == panel.monitor && contains(rect, point)
        })
        || state.model_popover.rect().is_some_and(|rect| {
            state.model_popover.monitor_idx == panel.monitor && contains(rect, point)
        })
    {
        return None;
    }
    if panel.under_toolbars
        && ([&state.toolbar, &state.action_toolbar]
            .into_iter()
            .any(|toolbar| toolbar.monitor_idx == panel.monitor && toolbar.hit_test(point).0)
            || state.settings_panel.monitor_idx == panel.monitor
                && state.settings_panel.hit_test(point).0)
    {
        return None;
    }
    Some(hit)
}

/// 【选区尺寸】【输入指针】按 state 当前全局指针更新字段拖选或弹层悬停，dirty 返回局部变化，返回是否阻止画布
pub(in crate::app) fn pointer(state: &mut EditorState, dirty: &mut u32) -> bool {
    if let Some(popup) = &mut state.selection_size.popup {
        if !popup.native && popup.monitor == state.input.pointer.monitor_idx {
            let changed = popup.pointer((
                state.input.pointer.local.0 - f64::from(popup.rect.left()),
                state.input.pointer.local.1 - f64::from(popup.rect.top()),
            ));
            if changed {
                let (monitor, rect) = (popup.monitor, popup.rect);
                state.damage_local(monitor, rect);
                crate::editor::dirty::mark_dirty(dirty, monitor);
            }
        }
        return true;
    }
    if super::field_pointer::drag(state, dirty) {
        return true;
    }
    let hovered = hit(state);
    let changed = state.selection_size.panel.as_mut().is_some_and(|panel| {
        let changed = panel.hovered != hovered;
        panel.hovered = hovered;
        changed
    });
    if changed {
        super::panels::damage(state, dirty);
    }
    hovered.is_some() || state.selection_size.pressed.is_some()
}

/// 【选区尺寸】【按钮分派】按 button 和 pressed 处理 state 面板输入，dirty 返回实际变化，右键不开始编辑或切换比例
pub(in crate::app) fn button(
    state: &mut EditorState,
    button: MouseButton,
    pressed: bool,
    dirty: &mut u32,
) -> bool {
    if let Some(owner) = state.selection_size.pressed {
        if !pressed && button == owner {
            super::field_clipboard::release(state, button, dirty);
            state.selection_size.pressed = None;
            state.selection_size.field_drag = None;
        }
        return true;
    }
    if let Some(popup) = &state.selection_size.popup {
        let point = state.input.pointer.local;
        let inside =
            popup.monitor == state.input.pointer.monitor_idx && contains(popup.rect, point);
        let event = if !inside && pressed {
            state.selection_size.pressed = Some(button);
            crate::ui::selection_size::ratio_popup::Event::Close
        } else {
            popup.button(
                (
                    point.0 - f64::from(popup.rect.left()),
                    point.1 - f64::from(popup.rect.top()),
                ),
                button,
                pressed,
            )
        };
        super::popup::chosen(state, event, dirty);
        return true;
    }
    let target = hit(state);
    if pressed && target.is_none() && matches!(button, MouseButton::Left) {
        super::apply::commit(state, dirty);
    }
    let Some(target) = target else { return false };
    if !pressed {
        return true;
    }
    state.selection_size.pressed = Some(button);
    if let Hit::Field(side) = target {
        if matches!(button, MouseButton::Left | MouseButton::Middle) {
            super::field_clipboard::press(state, side);
            if button == MouseButton::Left
                || state
                    .selection_size
                    .panel
                    .as_ref()
                    .unwrap()
                    .field(side)
                    .editing
            {
                super::field_pointer::press(state, side, button, dirty);
            }
        }
        return true;
    }
    if button != MouseButton::Left {
        return true;
    }
    match target {
        Hit::Field(_) => unreachable!(),
        Hit::Lock => {
            let panel = state.selection_size.panel.as_ref().unwrap();
            let ratio = if state.selection.aspect_ratio > 0.0 {
                0.0
            } else {
                f64::from(panel.values.0) / f64::from(panel.values.1)
            };
            super::apply::ratio(state, ratio, dirty);
        }
        Hit::Ratio => {
            super::popup::open(state, dirty);
        }
        Hit::Background => {}
    }
    true
}

/// 【选区尺寸】【字段滚轮】按 event 原始纵横轴符号修改 state 字段，dirty 返回立即应用的选区，不累计高分辨率余量
pub(in crate::app) fn scroll(state: &mut EditorState, event: ScrollEvent, dirty: &mut u32) -> bool {
    if state.selection_size.popup.is_some() {
        return true;
    }
    let Some(hit) = hit(state) else { return false };
    let Hit::Field(side) = hit else { return true };
    let axis = |axis: crate::types::ScrollAxis| {
        if axis.pixels != 0.0 && axis.pixels.is_finite() {
            axis.pixels
        } else if axis.value120 != 0 {
            f64::from(axis.value120)
        } else if axis.discrete != 0 {
            f64::from(axis.discrete)
        } else {
            f64::from(axis.angle_delta.unwrap_or(0))
        }
    };
    let vertical = axis(event.vertical);
    let delta = if vertical != 0.0 {
        vertical
    } else {
        axis(event.horizontal)
    };
    if delta != 0.0 {
        let amount = if delta > 0.0 { 1 } else { -1 } * if state.input.shift { 10 } else { 1 };
        let value = state
            .selection_size
            .panel
            .as_mut()
            .unwrap()
            .field_mut(side)
            .wheel(amount);
        if let Some(value) = value {
            super::apply::field(state, side, value, dirty);
        }
        super::panels::damage(state, dirty);
    }
    true
}
