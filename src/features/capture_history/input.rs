//! 【截图历史】【窗口输入】原版列表焦点、释放触发按钮与水平分隔拖动

use super::{
    layout::{Control, ROW, contains},
    splitter::Drag,
    window::{Focus, HistoryWindow},
};
use crate::{
    backend::wayland::window::{KeyInput, LEFT_BUTTON, RIGHT_BUTTON, WindowAction, WindowEvent},
    keys::{Key, Mods},
};
use std::time::{Duration, Instant};

impl HistoryWindow {
    /// 【截图历史】【焦点遍历】按 backward 在原版顺序中移动，跳过不可用按钮，无返回值
    fn focus_next(&mut self, backward: bool) {
        let mut order = vec![Focus::List];
        order.extend(
            [
                Control::Copy,
                Control::Edit,
                Control::Pin,
                Control::Save,
                Control::Delete,
                Control::Clear,
                Control::Enabled,
                Control::Close,
            ]
            .into_iter()
            .filter(|control| self.enabled(*control))
            .map(Focus::Control),
        );
        let current = order
            .iter()
            .position(|focus| *focus == self.focus)
            .unwrap_or(0);
        let next = if backward {
            (current + order.len() - 1) % order.len()
        } else {
            (current + 1) % order.len()
        };
        self.focus = order[next];
        self.pressed = None;
        self.keyboard_pressed = false;
    }

    /// 【截图历史】【键盘处理】处理 input，保留复制和删除快捷键；按钮 Space 等待释放，不把 Enter 当作编辑动作
    fn key(&mut self, input: KeyInput) -> WindowAction {
        if self.clear_confirmation {
            return match input.key {
                Some(Key::Escape) => self.control(Control::ConfirmCancel),
                Some(Key::Tab | Key::Left | Key::Right) => {
                    self.confirm_clear = !self.confirm_clear;
                    WindowAction::Redraw
                }
                Some(Key::Enter) => self.control(if self.confirm_clear {
                    Control::ConfirmClear
                } else {
                    Control::ConfirmCancel
                }),
                Some(Key::Space) => {
                    self.pressed = Some(if self.confirm_clear {
                        Control::ConfirmClear
                    } else {
                        Control::ConfirmCancel
                    });
                    self.keyboard_pressed = true;
                    WindowAction::Redraw
                }
                _ => WindowAction::None,
            };
        }
        if input.key == Some(Key::Escape) {
            return WindowAction::Close;
        }
        if input.key == Some(Key::Tab) && !input.mods.ctrl && !input.mods.alt && !input.mods.logo {
            self.focus_next(input.mods.shift);
            return WindowAction::Redraw;
        }
        if input.mods.ctrl
            && !input.mods.alt
            && !input.mods.shift
            && !input.mods.logo
            && input.key == Some(Key::Char('c'))
        {
            return self.control(Control::Copy);
        }
        if input.mods != Mods::default() {
            return WindowAction::None;
        }
        if input.key == Some(Key::Delete) {
            return self.control(Control::Delete);
        }
        if let Focus::Control(control) = self.focus {
            if input.key == Some(Key::Space) && self.enabled(control) {
                self.pressed = Some(control);
                self.keyboard_pressed = true;
                return WindowAction::Redraw;
            }
            return WindowAction::None;
        }
        let index = self.model.index();
        let rows = self.layout.rows();
        let next = match input.key {
            Some(Key::Up) => index.saturating_sub(1),
            Some(Key::Down) => index + 1,
            Some(Key::Home) => 0,
            Some(Key::End) => self.model.visible.len().saturating_sub(1),
            Some(Key::PageUp) => index.saturating_sub(rows),
            Some(Key::PageDown) => index + rows,
            _ => return WindowAction::None,
        };
        if self
            .model
            .select(next.min(self.model.visible.len().saturating_sub(1)), rows)
        {
            self.status = None;
        }
        self.previews();
        WindowAction::Redraw
    }

    /// 【截图历史】【键盘释放】按 input 完成本次 Space 按钮事务，返回窗口动作；没有按下事务时不执行操作
    fn key_release(&mut self, input: KeyInput) -> WindowAction {
        if input.key != Some(Key::Space) || !self.keyboard_pressed {
            return WindowAction::None;
        }
        self.keyboard_pressed = false;
        self.pressed
            .take()
            .map_or(WindowAction::None, |control| self.control(control))
    }

    /// 【截图历史】【鼠标按下】按 position 记录按钮或分隔事务，按钮不在按下时执行，返回窗口动作
    fn press(&mut self, position: (f64, f64)) -> WindowAction {
        if let Some(control) = self.layout.control(position, self.clear_confirmation) {
            self.last_click = None;
            if !self.enabled(control) {
                return WindowAction::None;
            }
            self.focus = Focus::Control(control);
            self.pressed = Some(control);
            self.keyboard_pressed = false;
            return WindowAction::Redraw;
        }
        if self.clear_confirmation {
            return WindowAction::None;
        }
        if contains(self.layout.splitter, position) {
            self.last_click = None;
            self.splitter_drag = Some(Drag::new(position, self.layout.list.width()));
            return WindowAction::None;
        }
        if let Some(bar) = self.scrollbar()
            && contains(bar.track, position)
        {
            self.last_click = None;
            self.focus = Focus::List;
            if contains(bar.handle, position) {
                self.scrollbar_drag = Some(super::scrollbar::Drag::new(position, &bar));
            } else {
                self.model.scroll = if position.1 < bar.handle.y() as f64 {
                    self.model.scroll.saturating_sub(bar.page)
                } else {
                    (self.model.scroll + bar.page).min(bar.maximum)
                };
                self.previews();
            }
            return WindowAction::Redraw;
        }
        if self.layout.resize_at(position) {
            self.last_click = None;
            return WindowAction::Resize;
        }
        if position.1 < 42.0 {
            self.last_click = None;
            return WindowAction::Move;
        }
        self.select_record(position, true)
    }

    /// 【截图历史】【鼠标释放】在 position 完成本次左键事务，仅在原按下按钮内释放才执行操作，返回窗口动作
    fn release(&mut self, position: (f64, f64)) -> WindowAction {
        self.splitter_drag = None;
        self.scrollbar_drag = None;
        if self.keyboard_pressed {
            return WindowAction::None;
        }
        let Some(control) = self.pressed.take() else {
            return WindowAction::None;
        };
        if self.layout.control(position, self.clear_confirmation) == Some(control) {
            self.control(control)
        } else {
            WindowAction::Redraw
        }
    }

    /// 【截图历史】【记录选择】按 position 选择记录，left 决定是否进入左键双击序列，返回重绘或编辑动作
    fn select_record(&mut self, position: (f64, f64), left: bool) -> WindowAction {
        if !left {
            self.last_click = None;
        }
        if !contains(self.layout.list, position) {
            self.last_click = None;
            return WindowAction::None;
        }
        if contains(self.layout.splitter, position)
            || self
                .scrollbar()
                .is_some_and(|bar| contains(bar.track, position))
        {
            self.last_click = None;
            return WindowAction::None;
        }
        let focus_changed = self.focus != Focus::List;
        self.focus = Focus::List;
        let y = position.1 - self.layout.list.y() as f64;
        let within = y % ROW as f64;
        if !(2.0..46.0).contains(&within) {
            self.last_click = None;
            return if focus_changed {
                WindowAction::Redraw
            } else {
                WindowAction::None
            };
        }
        let index = self.model.scroll + (y / ROW as f64) as usize;
        let Some(entry) = self.model.visible.get(index) else {
            self.last_click = None;
            return if focus_changed {
                WindowAction::Redraw
            } else {
                WindowAction::None
            };
        };
        let name = self.model.entries[*entry].name.clone();
        let double = left
            && self
                .last_click
                .as_ref()
                .is_some_and(|(time, previous, point)| {
                    time.elapsed() < Duration::from_millis(400)
                        && *previous == name
                        && (point.0 - position.0).abs() < 8.0
                        && (point.1 - position.1).abs() < 8.0
                });
        if left {
            self.last_click = Some((Instant::now(), name, position));
        }
        if self.model.select(index, self.layout.rows()) {
            self.status = None;
        }
        self.previews();
        if double {
            self.control(Control::Edit)
        } else {
            WindowAction::Redraw
        }
    }

    /// 【截图历史】【鼠标移动】按 position 更新悬停和分隔宽度，只有状态变化才返回重绘，不增加空闲循环
    fn motion(&mut self, position: (f64, f64)) -> WindowAction {
        let hovered = self.layout.control(position, self.clear_confirmation);
        let splitter = !self.clear_confirmation && contains(self.layout.splitter, position);
        let scrollbar = self
            .scrollbar()
            .is_some_and(|bar| contains(bar.track, position));
        let row = if !self.clear_confirmation
            && !splitter
            && !scrollbar
            && contains(self.layout.list, position)
        {
            Some(
                self.model.scroll
                    + ((position.1 - self.layout.list.y() as f64) / ROW as f64) as usize,
            )
        } else {
            None
        };
        let mut changed =
            self.hovered != hovered || self.splitter_hovered != splitter || self.hovered_row != row;
        self.hovered = hovered;
        self.splitter_hovered = splitter;
        self.hovered_row = row;
        if let Some(drag) = self.splitter_drag {
            let width = drag.width(position, self.layout.size.0 - 24.0);
            if width != self.layout.list.width() {
                self.left_width = width;
                self.relayout((self.layout.size.0 as u32, self.layout.size.1 as u32));
                self.previews();
                changed = true;
            }
        }
        if let Some(drag) = self.scrollbar_drag
            && let Some(bar) = self.scrollbar()
        {
            let scroll = drag.value(position, &bar);
            if scroll != self.model.scroll {
                self.model.scroll = scroll;
                self.previews();
                changed = true;
            }
        }
        if changed {
            WindowAction::Redraw
        } else {
            WindowAction::None
        }
    }
}

/// 【截图历史】【事件路由】将 event 交给 state，文件选择器和清空确认独占输入，返回窗口动作
pub(super) fn event(state: &mut HistoryWindow, event: WindowEvent) -> WindowAction {
    if state.picker.is_some() {
        if let WindowEvent::Key(input) = &event {
            state.ensure_fonts(&input.text);
        }
        if state.jobs.foreground() {
            return WindowAction::None;
        }
        let action = state
            .picker
            .as_mut()
            .unwrap()
            .event(&event, &mut state.fonts);
        return state.picker_action(action);
    }
    match event {
        WindowEvent::Key(input) => state.key(input),
        WindowEvent::KeyRelease(input) => state.key_release(input),
        WindowEvent::PointerButton {
            position,
            button: LEFT_BUTTON,
            pressed: true,
        } => state.press(position),
        WindowEvent::PointerButton {
            position,
            button: LEFT_BUTTON,
            pressed: false,
        } => state.release(position),
        WindowEvent::PointerButton {
            position,
            button: RIGHT_BUTTON,
            pressed: true,
        } if !state.clear_confirmation
            && state.pressed.is_none()
            && state.splitter_drag.is_none()
            && state.scrollbar_drag.is_none() =>
        {
            state.select_record(position, false)
        }
        WindowEvent::PointerMove { position } => state.motion(position),
        WindowEvent::PointerLeave => {
            let changed =
                state.hovered.is_some() || state.hovered_row.is_some() || state.splitter_hovered;
            state.hovered = None;
            state.hovered_row = None;
            state.splitter_hovered = false;
            if changed {
                WindowAction::Redraw
            } else {
                WindowAction::None
            }
        }
        WindowEvent::Scroll { position, delta }
            if !state.clear_confirmation && contains(state.layout.list, position) =>
        {
            let step = (delta.1 / ROW as f64).round() as isize;
            let step = if step == 0 && delta.1 != 0.0 {
                delta.1.signum() as isize
            } else {
                step
            };
            let old = state.model.scroll;
            state.model.scroll = (old as isize + step).max(0) as usize;
            state.model.scroll = state.model.scroll.min(
                state
                    .model
                    .visible
                    .len()
                    .saturating_sub(state.layout.rows()),
            );
            if state.model.scroll == old {
                WindowAction::None
            } else {
                state.previews();
                WindowAction::Redraw
            }
        }
        _ => WindowAction::None,
    }
}
