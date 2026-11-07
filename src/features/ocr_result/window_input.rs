//! 【文字识别】【窗口输入】分离左右键、正文选择和控件操作，菜单抓取期间不编辑正文

use super::{
    layout::{Layout, contains},
    window::{ResultWindow, View},
};
use crate::{
    backend::window::{KeyInput, LEFT_BUTTON, RIGHT_BUTTON, WindowAction, WindowEvent},
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
        if input.mods.ctrl
            && !input.mods.shift
            && !input.mods.alt
            && !input.mods.logo
            && input.key == Some(Key::Enter)
        {
            return super::translation::controller::start(self);
        }
        if input.key == Some(Key::Escape) {
            if input.mods != crate::keys::Mods::default() {
                return WindowAction::None;
            }
            if self.view == View::Table
                && self.table.as_mut().is_some_and(|table| table.finish(false))
            {
                return WindowAction::Redraw;
            }
            return WindowAction::Close;
        }
        if input.mods.ctrl
            && input.mods.shift
            && !input.mods.alt
            && !input.mods.logo
            && input.key == Some(Key::Char('c'))
        {
            let translation = self.view == View::Text
                && self.focus_target == super::control_input::FocusTarget::Translation
                && self.translation.state.visible;
            let editor = if translation {
                &self.translation.editor
            } else {
                &self.source
            };
            if !editor.has_non_whitespace() {
                return WindowAction::None;
            }
            let text = editor.text();
            self.copy_pane_text(
                &text,
                if translation {
                    super::feedback::Pane::Translation
                } else {
                    super::feedback::Pane::Source
                },
            );
            return WindowAction::Redraw;
        }
        if let Some(action) = super::translation::input::language_key(self, &input) {
            return action;
        }
        if let Some(action) = self.control_key(&input) {
            return action;
        }
        if self.view == View::Image {
            return WindowAction::None;
        }
        self.ensure_fonts(&input.text);
        let event = match self.view {
            View::Text => super::translation::input::key(self, &input),
            View::Table => self.table.as_mut().map_or(EditEvent::None, |table| {
                table.key(&mut self.fonts, self.layout.table_body, &input)
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
        if super::translation::input::select(self, position, drag, clicks) {
            return;
        }
        match self.view {
            View::Text => {
                let rect = self.layout.text_rect();
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
                    table.pointer(
                        &mut self.fonts,
                        self.layout.table_body,
                        position,
                        drag,
                        clicks,
                    );
                }
            }
            View::Image => {}
        }
    }

    /// 【文字识别】【鼠标按下】按 position 处理控件、标题移动、尺寸调整或正文编辑；返回窗口动作
    fn press(&mut self, position: (f64, f64)) -> WindowAction {
        if let Some(control) = self.layout.control(position, self.view) {
            return self.press_control(control, LEFT_BUTTON);
        }
        if self.layout.resize_at(position) {
            return WindowAction::Resize;
        }
        if contains(self.layout.titlebar.bounds, position) {
            return WindowAction::Move;
        }
        if self.view == View::Table
            && self
                .table
                .as_mut()
                .and_then(|table| {
                    table.bar_press(
                        self.layout.table_body,
                        position,
                        LEFT_BUTTON,
                        Instant::now(),
                    )
                })
                .is_some()
        {
            self.dragging = false;
            return WindowAction::Redraw;
        }
        if self.layout.translation.as_ref().is_some_and(|layout| {
            contains(layout.language, position)
                || contains(layout.frame, position)
                || contains(layout.splitter, position)
        }) {
            let clicks = self.clicks(position);
            if let Some(action) = super::translation::input::press(self, position, clicks) {
                return action;
            }
        }
        if contains(self.layout.body, position)
            && (self.view != View::Text || contains(self.layout.editor, position))
        {
            self.focus_control(super::control_input::FocusTarget::Content);
            self.control_press = None;
            let clicks = self.clicks(position);
            if self.view == View::Image {
                if clicks == 2 {
                    self.preview.reset();
                } else {
                    self.preview.press((
                        position.0 - f64::from(self.layout.body.x()),
                        position.1 - f64::from(self.layout.body.y()),
                    ));
                }
                return WindowAction::Redraw;
            }
            self.dragging = true;
            self.select(position, false, clicks);
            return WindowAction::Redraw;
        }
        WindowAction::None
    }
    /// 【文字识别】【窗口事件】处理 event；空闲、无状态变化时不安排重绘
    pub(super) fn input_event(&mut self, event: WindowEvent) -> WindowAction {
        if (self.source_menu.is_some() || self.translation.language_menu.is_some())
            && event.is_interaction()
        {
            return WindowAction::None;
        }
        match event {
            WindowEvent::Wake => {
                let now = Instant::now();
                let feedback = self.feedback.expire(now);
                let submenu = super::more_menu::wake(self, now);
                let translation = super::translation::controller::wake(self, now);
                let stacking = super::stacking::expire(self, now);
                let table = self
                    .table
                    .as_mut()
                    .is_some_and(|table| table.bar_wake(self.layout.table_body, now));
                if feedback || submenu || translation || stacking || table {
                    WindowAction::Redraw
                } else {
                    WindowAction::None
                }
            }
            WindowEvent::Resize { size } => {
                if let Some(table) = &mut self.table {
                    table.bars.cancel();
                }
                if (self.layout.size.0 >= 620.0) != (size.0 >= 620) {
                    self.translation.ratio = 0.6;
                }
                self.translation.language_menu = None;
                self.translation.splitter_drag = false;
                self.translation.layout_dirty = true;
                self.source_menu = None;
                self.more_menu.clear();
                self.layout = Layout::localized(
                    size,
                    self.table.is_some(),
                    true,
                    self.view,
                    self.feedback.label(super::feedback::Pane::Source),
                    self.feedback.label(super::feedback::Pane::Table),
                    self.view == View::Text && self.source.can_undo(),
                    self.status.is_some(),
                    self.language,
                );
                WindowAction::Redraw
            }
            WindowEvent::Focused(focused) => {
                self.focused = focused;
                self.dragging = false;
                self.preview.release();
                if !focused {
                    self.translation.language_menu = None;
                    self.translation.splitter_drag = false;
                    self.control_press = None;
                    self.source_menu = None;
                    self.more_menu.clear();
                    super::input_method::cancel(self);
                    if let Some(table) = &mut self.table {
                        table.bars.cancel();
                        table.finish(true);
                    }
                }
                WindowAction::Redraw
            }
            WindowEvent::TextInput(edit) => super::input_method::apply(self, edit),
            WindowEvent::Key(input) => self.key(input),
            WindowEvent::KeyRelease(input) => self.release_control_key(&input),
            WindowEvent::PointerButton {
                position,
                button: LEFT_BUTTON,
                pressed: true,
            } => self.press(position),
            WindowEvent::PointerButton {
                position,
                button,
                pressed: true,
            } if matches!(button, RIGHT_BUTTON | 0x112) => {
                if matches!(
                    self.control_press,
                    Some(super::control_input::Press::Pointer(_))
                ) {
                    return WindowAction::None;
                }
                if let Some(control) = self.layout.control(position, self.view) {
                    return self.press_control(control, button);
                }
                if self.view == View::Table
                    && self
                        .table
                        .as_mut()
                        .and_then(|table| {
                            table.bar_press(
                                self.layout.table_body,
                                position,
                                button,
                                Instant::now(),
                            )
                        })
                        .is_some()
                {
                    return WindowAction::Redraw;
                }
                if button == RIGHT_BUTTON
                    && self.view == View::Text
                    && contains(self.layout.editor, position)
                {
                    return super::source_menu::open(self, position);
                }
                if button == RIGHT_BUTTON
                    && let Some(layout) = &self.layout.translation
                {
                    let field = if contains(layout.frame, position) {
                        Some(super::translation::input::Field::Translation)
                    } else if !self.translation.state.busy()
                        && contains(layout.language_text, position)
                    {
                        Some(super::translation::input::Field::Language)
                    } else {
                        None
                    };
                    if let Some(field) = field {
                        return super::source_menu::open_field(self, position, field);
                    }
                }
                if button == RIGHT_BUTTON
                    && self.view == View::Image
                    && contains(self.layout.body, position)
                {
                    return super::source_image_menu::open(self, position);
                }
                WindowAction::None
            }
            WindowEvent::PointerButton {
                button,
                pressed: false,
                ..
            } if self.view == View::Table
                && self
                    .table
                    .as_mut()
                    .is_some_and(|table| table.bars.release(button)) =>
            {
                WindowAction::Redraw
            }
            WindowEvent::PointerButton {
                position,
                button: LEFT_BUTTON,
                pressed: false,
            } => {
                let split = std::mem::take(&mut self.translation.splitter_drag);
                self.dragging = false;
                if self.preview.release() {
                    return WindowAction::Redraw;
                }
                if split {
                    WindowAction::Redraw
                } else {
                    self.release_control(position)
                }
            }
            WindowEvent::PointerMove { position } => {
                let table_hover = self.view == View::Table
                    && self
                        .table
                        .as_mut()
                        .is_some_and(|table| table.bar_motion(self.layout.table_body, position));
                let hovered = self.layout.control(position, self.view);
                let changed = self.hovered != hovered;
                self.hovered = hovered;
                if super::translation::input::split_motion(self, position) {
                    return WindowAction::Redraw;
                }
                if self.view == View::Image
                    && self.preview.motion(
                        (
                            self.layout.body.width() as i32,
                            self.layout.body.height() as i32,
                        ),
                        (self.image.width(), self.image.height()),
                        (
                            position.0 - f64::from(self.layout.body.x()),
                            position.1 - f64::from(self.layout.body.y()),
                        ),
                    )
                {
                    return WindowAction::Redraw;
                }
                if self.dragging {
                    self.select(position, true, 1);
                }
                if changed || self.dragging || table_hover {
                    WindowAction::Redraw
                } else {
                    WindowAction::None
                }
            }
            WindowEvent::PointerLeave => {
                if let Some(table) = &mut self.table {
                    table.bars.hovered = None;
                }
                self.hovered = None;
                self.dragging = false;
                WindowAction::Redraw
            }
            WindowEvent::Scroll { position, delta }
                if self
                    .layout
                    .translation
                    .as_ref()
                    .is_some_and(|layout| contains(layout.language, position)) =>
            {
                if self.translation.state.busy() || delta.1 == 0.0 {
                    return WindowAction::None;
                }
                let current = self.translation.target();
                let current = crate::translation::language::OPTIONS
                    .iter()
                    .position(|value| *value == current)
                    .unwrap_or(0);
                super::translation::controller::choose(
                    self,
                    if delta.1 > 0.0 {
                        (current + 1).min(8)
                    } else {
                        current.saturating_sub(1)
                    },
                )
            }
            WindowEvent::Scroll { position, delta } if contains(self.layout.body, position) => {
                if self.view == View::Image {
                    return if self.preview.wheel(
                        (
                            self.layout.body.width() as i32,
                            self.layout.body.height() as i32,
                        ),
                        (self.image.width(), self.image.height()),
                        (
                            position.0 - f64::from(self.layout.body.x()),
                            position.1 - f64::from(self.layout.body.y()),
                        ),
                        -delta.1,
                    ) {
                        WindowAction::Redraw
                    } else {
                        WindowAction::None
                    };
                }
                match self.view {
                    View::Text => {
                        if self
                            .layout
                            .translation
                            .as_ref()
                            .is_some_and(|layout| contains(layout.frame, position))
                        {
                            self.translation
                                .editor
                                .scroll(&mut self.fonts, delta.1 as f32);
                        } else {
                            self.source.scroll(&mut self.fonts, delta.1 as f32);
                        }
                    }
                    View::Table => {
                        return self.scroll_input(crate::backend::window::ScrollInput {
                            position,
                            ..super::table_scroll_model::legacy(delta, crate::keys::Mods::default())
                        });
                    }
                    View::Image => return WindowAction::None,
                }
                WindowAction::Redraw
            }
            WindowEvent::Wheel(input) => self.scroll_input(input),
            _ => WindowAction::None,
        }
    }
}
