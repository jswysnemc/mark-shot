//! 【X11覆盖层】【输入循环】将原生窗口事件转换为编辑器事件并处理曝光和尺寸变化

use super::*;
use crate::{
    keys::Chord,
    types::{OverlayEvent, ScrollEvent},
};
use std::time::{Duration, Instant};
use x11rb::{connection::Connection, protocol::Event};

impl Overlay {
    /// 【X11覆盖层】【有界等待】按timeout等待下一条业务事件，返回超时通知或连接错误
    pub(super) fn event(&mut self, timeout: Option<Duration>) -> crate::AppResult<OverlayEvent> {
        let deadline = timeout.map(|timeout| Instant::now() + timeout);
        loop {
            if super::super::wait::theme_changed(self.theme.as_ref()) {
                return Ok(OverlayEvent::ThemeChanged);
            }
            if let Some(event) = self.pending.pop_front() {
                return Ok(event);
            }
            while let Some(event) = self.display.connection.poll_for_event()? {
                self.keyboard.update(&self.display, &event)?;
                if self.ime.filter(&event) {
                    self.collect_ime()?;
                    if let Some(event) = self.pending.pop_front() {
                        return Ok(event);
                    }
                    continue;
                }
                if self.popup.is_none()
                    && let Event::KeyPress(key) | Event::KeyRelease(key) = &event
                    && self.ime.forward(key)?
                {
                    continue;
                }
                if self.popup_input(&event)? {
                    if let Some(event) = self.pending.pop_front() {
                        return Ok(event);
                    }
                    continue;
                }
                if let Some(event) = self.translate(event)? {
                    return Ok(event);
                }
            }
            let timeout =
                deadline.map(|deadline| deadline.saturating_duration_since(Instant::now()));
            if timeout.is_some_and(|timeout| timeout.is_zero()) {
                return Ok(OverlayEvent::Tick);
            }
            if super::super::wait::wait_with_theme(
                &self.display,
                &self.wake,
                timeout,
                self.theme.as_ref(),
            )? {
                return Ok(OverlayEvent::Tick);
            }
        }
    }

    /// 【X11覆盖层】【协议分派】按event查找目标表面并转换输入，返回业务事件或已消费状态
    pub(super) fn translate(&mut self, event: Event) -> crate::AppResult<Option<OverlayEvent>> {
        use super::super::input;
        let window = match &event {
            Event::MotionNotify(event) => event.event,
            Event::EnterNotify(event) => event.event,
            Event::ButtonPress(event) | Event::ButtonRelease(event) => event.event,
            Event::KeyPress(event) | Event::KeyRelease(event) => event.event,
            Event::FocusIn(event) | Event::FocusOut(event) => event.event,
            Event::Expose(event) => event.window,
            Event::ConfigureNotify(event) => event.window,
            Event::ClientMessage(event) => event.window,
            Event::Error(error) => return Err(format!("X11 protocol error: {error:?}").into()),
            _ => return Ok(None),
        };
        let Some(index) = self
            .surfaces
            .iter()
            .position(|(_, surface)| surface.window == window)
        else {
            return Ok(None);
        };
        let offset = self.input_offset(index);
        let result = match event {
            Event::MotionNotify(event) => {
                self.cursor_output = Some(index);
                Some(OverlayEvent::PointerMove {
                    monitor_idx: index,
                    x: f64::from(event.event_x) + offset.0,
                    y: f64::from(event.event_y) + offset.1,
                    history: Vec::new(),
                })
            }
            Event::EnterNotify(event) => {
                self.cursor_output = Some(index);
                Some(OverlayEvent::PointerMove {
                    monitor_idx: index,
                    x: f64::from(event.event_x) + offset.0,
                    y: f64::from(event.event_y) + offset.1,
                    history: Vec::new(),
                })
            }
            Event::ButtonPress(event) | Event::ButtonRelease(event) => {
                let pressed = event.response_type == x11rb::protocol::xproto::BUTTON_PRESS_EVENT;
                self.pending.push_back(OverlayEvent::PointerMove {
                    monitor_idx: index,
                    x: f64::from(event.event_x) + offset.0,
                    y: f64::from(event.event_y) + offset.1,
                    history: Vec::new(),
                });
                if let Some(button) = input::button(event.detail) {
                    self.pending.push_back(OverlayEvent::PointerButton {
                        button,
                        pressed,
                        serial: None,
                        timestamp: Some(event.time),
                    });
                } else if pressed && matches!(event.detail, 4 | 5) {
                    self.pending
                        .push_back(OverlayEvent::Scroll(ScrollEvent::wheel(
                            if event.detail == 4 { -120 } else { 120 },
                        )));
                }
                self.pending.pop_front()
            }
            Event::KeyPress(event) => {
                let key = self.keyboard.key(&event, true);
                let mods = self.keyboard.modifiers();
                self.pending.push_back(OverlayEvent::ModifiersChanged {
                    ctrl: mods.ctrl,
                    shift: mods.shift,
                });
                Some(OverlayEvent::Key {
                    chord: key.key.map(|value| Chord {
                        key: value,
                        mods: key.mods,
                    }),
                    text: (!key.text.is_empty()).then_some(key.text),
                })
            }
            Event::KeyRelease(event) => {
                self.keyboard.key(&event, false);
                let mods = self.keyboard.modifiers();
                Some(OverlayEvent::ModifiersChanged {
                    ctrl: mods.ctrl,
                    shift: mods.shift,
                })
            }
            Event::FocusIn(event) if event.mode == x11rb::protocol::xproto::NotifyMode::NORMAL => {
                self.text_focus(Some(index))?;
                Some(OverlayEvent::Focus { monitor_idx: index })
            }
            Event::FocusOut(event) if event.mode == x11rb::protocol::xproto::NotifyMode::NORMAL => {
                self.text_focus(None)?;
                Some(OverlayEvent::FocusLost { monitor_idx: index })
            }
            Event::Expose(_) => {
                self.surfaces[index].1.repaint()?;
                None
            }
            Event::ConfigureNotify(event) if self.image_size.is_some() => {
                let size = (u32::from(event.width), u32::from(event.height));
                if self.image_size != Some(size) {
                    self.image_size = Some(size);
                    self.surfaces[index].1.size = (event.width, event.height);
                    Some(OverlayEvent::CanvasResized { size })
                } else {
                    None
                }
            }
            Event::ClientMessage(event)
                if event.type_ == self.display.atom("WM_PROTOCOLS")?
                    && event.data.as_data32()[0] == self.display.atom("WM_DELETE_WINDOW")? =>
            {
                Some(OverlayEvent::EscapePressed)
            }
            _ => None,
        };
        Ok(result)
    }
}
