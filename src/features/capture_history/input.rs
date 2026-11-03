//! 【截图历史】【窗口输入】搜索字段优先、列表导航与确认弹层隔离

use super::{
    layout::{Control, ROW, contains},
    window::{Focus, HistoryWindow},
};
use crate::{
    backend::wayland::window::{KeyInput, LEFT_BUTTON, WindowAction, WindowEvent},
    keys::Key,
};
use std::time::{Duration, Instant};

impl HistoryWindow {
    /// 【截图历史】【搜索同步】将搜索输入应用到浏览模型并安排当前预览；不进行 OCR
    fn sync_search(&mut self) {
        self.model.search(&self.search.text());
        self.previews();
    }

    /// 【截图历史】【键盘处理】处理 input；搜索和数量字段优先，清空确认默认取消
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
                _ => WindowAction::None,
            };
        }
        self.ensure_fonts(&input.text);
        if input.mods.ctrl && !input.mods.alt && input.key == Some(Key::Char('f')) {
            self.focus = Focus::Search;
            self.search.select_all(&mut self.fonts);
            return WindowAction::Redraw;
        }
        if input.key == Some(Key::Escape) {
            if self.focus == Focus::Limit {
                self.limit
                    .set(&self.settings.limit.to_string(), &mut self.fonts, false);
                self.focus = Focus::List;
                return WindowAction::Redraw;
            }
            if self.focus == Focus::Search && !self.search.text().is_empty() {
                self.search.set("", &mut self.fonts, false);
                self.sync_search();
                self.focus = Focus::List;
                return WindowAction::Redraw;
            }
            return WindowAction::Close;
        }
        if input.key == Some(Key::Tab) {
            self.focus = match (self.focus, input.mods.shift) {
                (Focus::List, false) | (Focus::Limit, true) => Focus::Search,
                (Focus::Search, false) | (Focus::List, true) => Focus::Limit,
                _ => Focus::List,
            };
            return WindowAction::Redraw;
        }
        if input.key == Some(Key::Enter) && self.focus != Focus::List {
            if self.focus == Focus::Limit {
                self.commit_limit();
            } else {
                self.focus = Focus::List;
            }
            return WindowAction::Redraw;
        }
        if self.focus == Focus::Search {
            let changed = self.search.key(&mut self.fonts, &input);
            if changed {
                self.sync_search();
            }
            return if changed {
                WindowAction::Redraw
            } else {
                WindowAction::None
            };
        }
        if self.focus == Focus::Limit {
            let changed = self.limit.key(&mut self.fonts, &input);
            return if changed {
                WindowAction::Redraw
            } else {
                WindowAction::None
            };
        }
        if input.mods.ctrl && !input.mods.alt {
            let control = match input.key {
                Some(Key::Char('c')) => Some(Control::Copy),
                Some(Key::Char('e')) => Some(Control::Edit),
                Some(Key::Char('p')) => Some(Control::Pin),
                Some(Key::Char('s')) => Some(Control::Save),
                Some(Key::Char('r')) => Some(Control::Refresh),
                Some(Key::Delete) if input.mods.shift => Some(Control::Clear),
                _ => None,
            };
            if let Some(control) = control {
                return self.control(control);
            }
        }
        match input.key {
            Some(Key::Enter) => return self.control(Control::Edit),
            Some(Key::Delete | Key::Backspace) => return self.control(Control::Delete),
            Some(Key::F(5)) => return self.control(Control::Refresh),
            _ => {}
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
        self.model
            .select(next.min(self.model.visible.len().saturating_sub(1)), rows);
        self.previews();
        WindowAction::Redraw
    }

    /// 【截图历史】【鼠标按下】处理 position 的字段、记录和控件；确认弹层不允许底层点击穿透
    fn press(&mut self, position: (f64, f64)) -> WindowAction {
        if let Some(control) = self.layout.control(position, self.clear_confirmation) {
            if self.focus == Focus::Limit
                && !self.clear_confirmation
                && !matches!(control, Control::Minus | Control::Plus | Control::Close)
            {
                self.commit_limit();
            }
            self.focus = Focus::List;
            return self.control(control);
        }
        if self.clear_confirmation {
            return WindowAction::None;
        }
        if self.layout.resize_at(position) {
            return WindowAction::Resize;
        }
        if position.1 < 36.0 {
            return WindowAction::Move;
        }
        if contains(self.layout.search, position) {
            self.focus = Focus::Search;
            self.dragging = true;
            let rect = self.layout.search_text;
            self.search.pointer(
                &mut self.fonts,
                (position.0 - rect.x() as f64, position.1 - rect.y() as f64),
                false,
            );
            return WindowAction::Redraw;
        }
        if contains(self.layout.limit, position) {
            self.focus = Focus::Limit;
            self.limit.select_all(&mut self.fonts);
            return WindowAction::Redraw;
        }
        if self.focus == Focus::Limit {
            self.commit_limit();
        }
        self.focus = Focus::List;
        if contains(self.layout.list, position) {
            let index = self.model.scroll
                + ((position.1 - self.layout.list.y() as f64) / ROW as f64) as usize;
            if index >= self.model.visible.len() {
                return WindowAction::None;
            }
            let name = self.model.entries[self.model.visible[index]].name.clone();
            let double = self.last_click.as_ref().is_some_and(|(time, previous)| {
                time.elapsed() < Duration::from_millis(400) && *previous == name
            });
            self.last_click = Some((Instant::now(), name));
            self.model.select(index, self.layout.rows());
            self.previews();
            if double {
                return self.control(Control::Edit);
            }
            return WindowAction::Redraw;
        }
        WindowAction::None
    }
}

/// 【截图历史】【事件路由】处理 state 上的 event；文件选择对话框独占输入，其余事件进入历史界面
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
        WindowEvent::PointerButton {
            position,
            button: LEFT_BUTTON,
            pressed: true,
        } => state.press(position),
        WindowEvent::PointerButton { pressed: false, .. } => {
            state.dragging = false;
            WindowAction::None
        }
        WindowEvent::PointerMove { position } => {
            let hovered = state.layout.control(position, state.clear_confirmation);
            let changed = state.hovered != hovered;
            state.hovered = hovered;
            if state.dragging && state.focus == Focus::Search && !state.clear_confirmation {
                let rect = state.layout.search_text;
                state.search.pointer(
                    &mut state.fonts,
                    (position.0 - rect.x() as f64, position.1 - rect.y() as f64),
                    true,
                );
            }
            if changed || state.dragging {
                WindowAction::Redraw
            } else {
                WindowAction::None
            }
        }
        WindowEvent::PointerLeave => {
            state.dragging = false;
            state.hovered = None;
            WindowAction::Redraw
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
                return WindowAction::None;
            }
            state.previews();
            WindowAction::Redraw
        }
        _ => WindowAction::None,
    }
}
