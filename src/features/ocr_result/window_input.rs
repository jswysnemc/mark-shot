//! 【文字识别】【窗口输入】分离左右键、正文选择和控件操作，菜单抓取期间不编辑正文

use super::{
    layout::{Control, Layout, contains},
    window::{ResultWindow, View},
};
use crate::{
    backend::wayland::window::{KeyInput, LEFT_BUTTON, RIGHT_BUTTON, WindowAction, WindowEvent},
    keys::Key,
    ui::text_editor::EditEvent,
};
use std::time::{Duration, Instant};

impl ResultWindow {
    /// 【文字识别】【结果快捷键】处理 input 的关闭、全文复制与编辑动作；原图页不接收文字编辑
    fn key(&mut self, input: KeyInput) -> WindowAction {
        if !self.focused {
            return WindowAction::None;
        }
        if input.key == Some(Key::Escape) {
            if self.menu {
                self.menu = false;
                return WindowAction::Redraw;
            }
            if self.view == View::Table
                && self.table.as_mut().is_some_and(|table| table.finish(false))
            {
                return WindowAction::Redraw;
            }
            return WindowAction::Close;
        }
        if let Some(control) = self.control_focus {
            if input.key == Some(Key::Tab) && !input.mods.ctrl && !input.mods.alt {
                self.control_focus = None;
                return WindowAction::Redraw;
            }
            if matches!(input.key, Some(Key::Enter | Key::Char(' ')))
                && !input.mods.ctrl
                && !input.mods.alt
            {
                return self.control(control);
            }
            if !(input.mods.ctrl && input.mods.shift && input.key == Some(Key::Char('c'))) {
                return WindowAction::None;
            }
        }
        self.ensure_fonts(&input.text);
        if input.mods.ctrl && input.mods.shift && input.key == Some(Key::Char('c')) {
            self.copy_text(&self.source.text());
            return WindowAction::Redraw;
        }
        let event = match self.view {
            View::Text => self.source.key(&mut self.fonts, &input),
            View::Table => self.table.as_mut().map_or(EditEvent::None, |table| {
                table.key(&mut self.fonts, self.layout.body, &input)
            }),
            View::Image => return WindowAction::None,
        };
        self.edit_event(event)
    }

    /// 【文字识别】【连续点击】根据 position 与上次点击时间返回单击、双击或三击次数
    fn clicks(&mut self, position: (f64, f64)) -> u8 {
        let now = Instant::now();
        let count = self
            .last_click
            .filter(|(time, previous, _)| {
                now.duration_since(*time) <= Duration::from_millis(450)
                    && (previous.0 - position.0).hypot(previous.1 - position.1) < 5.0
            })
            .map_or(1, |(_, _, count)| if count == 3 { 1 } else { count + 1 });
        self.last_click = Some((now, position, count));
        count
    }

    /// 【文字识别】【正文选择】将 position 转换为当前页本地坐标；drag、clicks 控制文字选择范围
    fn select(&mut self, position: (f64, f64), drag: bool, clicks: u8) {
        match self.view {
            View::Text => {
                let rect = self.layout.editor;
                self.source
                    .resize(&mut self.fonts, rect.width(), rect.height());
                self.source.pointer(
                    &mut self.fonts,
                    (
                        position.0 - f64::from(rect.x()),
                        position.1 - f64::from(rect.y()),
                    ),
                    drag,
                    clicks,
                );
            }
            View::Table => {
                if let Some(table) = &mut self.table {
                    table.pointer(&mut self.fonts, self.layout.body, position, drag, clicks);
                }
            }
            View::Image => {}
        }
    }

    /// 【文字识别】【鼠标按下】按 position 处理控件、标题移动、尺寸调整或正文编辑；返回窗口动作
    fn press(&mut self, position: (f64, f64)) -> WindowAction {
        if let Some(control) = self.layout.control(position, self.menu) {
            if matches!(control, Control::Copy | Control::Undo) {
                super::input_method::cancel(self);
                self.control_focus = Some(control);
            }
            return self.control(control);
        }
        if self.menu {
            self.menu = false;
            return WindowAction::Redraw;
        }
        if self.layout.resize_at(position) {
            return WindowAction::Resize;
        }
        if position.1 < 36.0 {
            return WindowAction::Move;
        }
        if contains(self.layout.body, position) {
            self.control_focus = None;
            self.dragging = self.view != View::Image;
            let clicks = self.clicks(position);
            self.select(position, false, clicks);
            return WindowAction::Redraw;
        }
        WindowAction::None
    }
    /// 【文字识别】【窗口事件】处理 event；空闲、无状态变化时不安排重绘
    pub(super) fn input_event(&mut self, event: WindowEvent) -> WindowAction {
        if self.source_menu.is_some() && event.is_interaction() {
            return WindowAction::None;
        }
        match event {
            WindowEvent::Resize { size } => {
                self.source_menu = None;
                self.layout = Layout::new(size, self.table.is_some());
                WindowAction::Redraw
            }
            WindowEvent::Focused(focused) => {
                self.focused = focused;
                self.dragging = false;
                if !focused {
                    self.source_menu = None;
                    super::input_method::cancel(self);
                    if let Some(table) = &mut self.table {
                        table.finish(true);
                    }
                }
                WindowAction::Redraw
            }
            WindowEvent::TextInput(edit) => super::input_method::apply(self, edit),
            WindowEvent::Key(input) => self.key(input),
            WindowEvent::PointerButton {
                position,
                button: LEFT_BUTTON,
                pressed: true,
            } => self.press(position),
            WindowEvent::PointerButton {
                position,
                button: RIGHT_BUTTON,
                pressed: true,
            } if self.view == View::Text && contains(self.layout.editor, position) => {
                super::source_menu::open(self, position)
            }
            WindowEvent::PointerButton {
                button: LEFT_BUTTON,
                pressed: false,
                ..
            } => {
                self.dragging = false;
                WindowAction::None
            }
            WindowEvent::PointerMove { position } => {
                let hovered = self.layout.control(position, self.menu);
                let changed = self.hovered != hovered;
                self.hovered = hovered;
                if self.dragging {
                    self.select(position, true, 1);
                }
                if changed || self.dragging {
                    WindowAction::Redraw
                } else {
                    WindowAction::None
                }
            }
            WindowEvent::PointerLeave => {
                self.hovered = None;
                self.dragging = false;
                WindowAction::Redraw
            }
            WindowEvent::Scroll { position, delta } if contains(self.layout.body, position) => {
                match self.view {
                    View::Text => self.source.scroll(&mut self.fonts, delta.1 as f32),
                    View::Table => {
                        if let Some(table) = &mut self.table {
                            table.scroll(self.layout.body, delta);
                        }
                    }
                    View::Image => return WindowAction::None,
                }
                WindowAction::Redraw
            }
            _ => WindowAction::None,
        }
    }
}
