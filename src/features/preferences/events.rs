//! 【设置窗口】【输入分派】原生窗口事件与对话框结果独立于配置状态

use super::{
    catalog::{ENTRIES, Kind},
    window::{Control, Preferences},
};
use crate::{
    backend::window::{self, WindowAction, WindowContent, WindowCursor, WindowEvent},
    keys::Key,
};

impl WindowContent for Preferences {
    /// 【设置窗口】【系统关闭】无参数，关闭前提交输入，失败保留窗口和草稿，返回是否允许关闭
    fn close_requested(&mut self) -> bool {
        self.commit_pending()
    }
    /// 【设置窗口】【资源对话框】无参数，取出待打开的 OCR 资源管理窗口，返回可选对话框
    fn take_dialog(&mut self) -> Option<window::WindowDialog> {
        self.dialog.take()
    }
    /// 【设置窗口】【资源返回】按对话框关闭事件刷新安装摘要，返回重绘动作，不启动下载
    fn dialog_closed(&mut self, id: u64, _accepted: bool) -> WindowAction {
        if id == super::colors::DIALOG_ID {
            super::colors::finish(self);
        }
        self.refresh_resources();
        WindowAction::Redraw
    }
    /// 【设置窗口】【动画调度】无参数，返回工具栏避让动画下一帧期限，静止时没有周期工作
    fn next_deadline(&self) -> Option<std::time::Instant> {
        self.toolbar.animation.deadline()
    }
    /// 【翻译设置】【完成通知】保存 wake 用于后台目录查询完成后唤醒窗口，无返回值
    fn connected(&mut self, wake: window::WindowWake) {
        self.desktop_watch = crate::features::desktop::status::watch(&self.path, wake.clone()).ok();
        self.theme_watch = Some(crate::theme::watch_system(wake.clone()));
        self.translation.wake = Some(wake);
    }
    /// 【设置窗口】【首帧确认】无参数，按启动约定通知父进程窗口已经映射，无返回值
    fn mapped(&mut self) {
        if self.ready {
            use std::io::Write;
            let _ = std::io::stdout().write_all(b"\n");
            self.ready = false;
        }
    }
    /// 【设置窗口】【画面输出】以 scale 将当前状态绘制到 canvas，静止时不发起周期绘制
    fn draw(&mut self, canvas: &mut tiny_skia::Pixmap, scale: f32) {
        super::draw::draw(self, canvas, scale);
        super::dropdown::draw::draw(self, canvas, scale);
        crate::ui::window_frame::draw(canvas, scale);
    }
    /// 【设置窗口】【输入路由】按 event 更新当前控件和配置，返回必要的重绘或关闭动作
    fn event(&mut self, event: WindowEvent) -> WindowAction {
        if let WindowEvent::PointerButton {
            position,
            button: window::LEFT_BUTTON,
            pressed: true,
        } = event
            && let Some(edge) = super::layout::resize_edge(self.size, position)
        {
            return WindowAction::ResizeEdge(edge);
        }
        if matches!(event, WindowEvent::Wake) {
            self.desktop_status = crate::features::desktop::status::read(&self.path);
            super::translation::input::event(self, &event);
            return WindowAction::Redraw;
        }
        if let Some(action) = super::dropdown::input::event(self, &event) {
            return action;
        }
        let shell_hover_changed = match &event {
            WindowEvent::PointerMove { position } => self.hit(self.pointer) != self.hit(*position),
            _ => false,
        };
        if (self.translation_page() || matches!(event, WindowEvent::Wake))
            && let Some(action) = super::translation::input::event(self, &event)
        {
            return if shell_hover_changed && matches!(action, WindowAction::None) {
                WindowAction::Redraw
            } else {
                action
            };
        }
        if self.toolbar_page()
            && let Some(action) = super::toolbar::input::event(self, &event)
        {
            return if shell_hover_changed && matches!(action, WindowAction::None) {
                WindowAction::Redraw
            } else {
                action
            };
        }
        match event {
            WindowEvent::Resize { size } => {
                self.size = size;
                self.toolbar.scroll = self
                    .toolbar
                    .scroll
                    .min(super::toolbar::layout::Layout::new(self).maximum());
                self.scroll = self.scroll.min(super::layout::Layout::new(self).maximum);
            }
            WindowEvent::PointerMove { position } => {
                if self.pressed.is_some() && self.edit_pointer(position, true) {
                    self.pointer = position;
                    return WindowAction::Redraw;
                }
                if let Some(drag) = self.scrollbar_drag
                    && let Some(bar) = self.scrollbar()
                {
                    self.scroll = drag.value(position, &bar);
                    self.pointer = position;
                    return WindowAction::Redraw;
                }
                let before = self.hit(self.pointer);
                self.pointer = position;
                if !shell_hover_changed && before == self.hit(position) {
                    return WindowAction::None;
                }
            }
            WindowEvent::PointerButton {
                position,
                button: window::LEFT_BUTTON,
                pressed,
            } => {
                let hit = self.hit(position);
                if !pressed && self.scrollbar_drag.take().is_some() {
                    self.pressed = None;
                    return WindowAction::Redraw;
                }
                if pressed {
                    self.edit_pointer(position, false);
                    if self.press_scrollbar(position) {
                        return WindowAction::Redraw;
                    }
                    if position.0 >= self.size.0 as f64 - 18.0
                        && position.1 >= self.size.1 as f64 - 18.0
                    {
                        return WindowAction::Resize;
                    }
                    if hit.is_none() && position.1 < 48.0 {
                        return WindowAction::Move;
                    }
                    self.pressed = hit;
                } else if let Some(control) =
                    self.pressed.take().filter(|control| Some(*control) == hit)
                {
                    if control == Control::Close {
                        return if self.commit_pending() {
                            WindowAction::Close
                        } else {
                            WindowAction::Redraw
                        };
                    }
                    self.activate(control);
                }
            }
            WindowEvent::Scroll { position, delta } => {
                if self.scroll_number(position, delta.1) {
                    return WindowAction::Redraw;
                }
                self.scroll_rows(if delta.1 > 0.0 {
                    1
                } else if delta.1 < 0.0 {
                    -1
                } else {
                    0
                });
            }
            WindowEvent::Key(input) => {
                if input.key != Some(Key::Escape)
                    && let Some((index, _)) = &self.editing
                    && matches!(ENTRIES[*index].kind, Kind::Shortcut)
                {
                    self.record_shortcut(*index, &input);
                    return WindowAction::Redraw;
                }
                if input.key == Some(Key::Escape) {
                    if self.back() {
                        return WindowAction::Redraw;
                    }
                    self.shortcut_pause = None;
                    if self.editing.take().is_none() && !self.search_focus {
                        return if self.commit_pending() {
                            WindowAction::Close
                        } else {
                            WindowAction::Redraw
                        };
                    }
                    self.search_focus = false;
                } else if input.mods.ctrl && input.key == Some(Key::Char('f')) {
                    if !self.commit_pending() {
                        return WindowAction::Redraw;
                    }
                    self.back();
                    self.search_focus = true;
                    self.editing = None;
                    self.shortcut_pause = None;
                    self.search.select_all(&mut self.fonts);
                } else if self.search_focus {
                    self.search.key(&mut self.fonts, &input);
                    self.scroll = 0;
                } else if input.key == Some(Key::Tab) {
                    if self.commit_pending() {
                        self.advance_focus(input.mods.shift);
                    }
                } else if input.key == Some(Key::Enter) && self.editing.is_some() {
                    self.apply();
                } else if let Some((index, field)) = &mut self.editing {
                    if matches!(ENTRIES[*index].kind, Kind::Shortcut) {
                        let index = *index;
                        self.record_shortcut(index, &input);
                    } else {
                        field.key(&mut self.fonts, &input);
                    }
                } else if self.scroll_key(input.key) {
                    self.focus = None;
                } else if input.key == Some(Key::Enter)
                    && let Some(index) = self.focus
                {
                    self.activate(Control::Row(index));
                }
            }
            WindowEvent::Focused(false) => {
                self.pressed = None;
                self.scrollbar_drag = None;
            }
            _ => return WindowAction::None,
        }
        WindowAction::Redraw
    }
    /// 【设置窗口】【指针反馈】按 position 返回文本或按钮光标，无状态修改
    fn cursor(&self, position: (f64, f64)) -> WindowCursor {
        if let Some(edge) = super::layout::resize_edge(self.size, position) {
            use window::WindowEdge;
            return match edge {
                WindowEdge::Top | WindowEdge::Bottom => WindowCursor::VerticalResize,
                WindowEdge::Left | WindowEdge::Right => WindowCursor::HorizontalResize,
                _ => WindowCursor::Resize,
            };
        }
        if let Some(menu) = &self.dropdown {
            return match super::dropdown::layout::Layout::new(menu, self.size).hit(position) {
                Some(super::dropdown::layout::Hit::Search) => WindowCursor::Text,
                Some(_) => WindowCursor::Pointer,
                None => WindowCursor::Default,
            };
        }
        if self.translation_page()
            && super::translation::layout::Layout::new(self)
                .hit(position)
                .is_some()
        {
            return WindowCursor::Pointer;
        }
        if self.toolbar_page() {
            if self.toolbar.drag.as_ref().is_some_and(|drag| drag.active) {
                return WindowCursor::Grabbing;
            }
            if let Some(hit) = super::toolbar::layout::Layout::new(self).hit(position) {
                return if matches!(hit, super::toolbar::layout::Hit::Item(..)) {
                    WindowCursor::Grab
                } else {
                    WindowCursor::Pointer
                };
            }
        }
        if position.0 >= self.size.0 as f64 - 18.0 && position.1 >= self.size.1 as f64 - 18.0 {
            return WindowCursor::Resize;
        }
        match self.hit(position) {
            Some(Control::Search) => WindowCursor::Text,
            Some(_) => WindowCursor::Pointer,
            None => WindowCursor::Default,
        }
    }
}
