//! 【X11普通窗口】【输入分派】输入优先交给模态窗口及菜单，窗口动作由父内容统一处理

use super::*;
use crate::backend::x11::input;

impl Node {
    /// 【X11普通窗口】【事件转换】按event分派窗口、菜单或模态内容，返回协议错误
    pub(super) fn event(&mut self, event: Event) -> crate::AppResult {
        self.keyboard.update(&self.surface.display, &event)?;
        if self.ime_event(&event)? {
            return Ok(());
        }
        if let Some((_, child)) = &mut self.dialog {
            child.event(event.clone())?;
        }
        if self.dialog.is_none() && self.popup_event(&event)? {
            return Ok(());
        }
        let window = match &event {
            Event::KeyPress(event) | Event::KeyRelease(event) => event.event,
            Event::ButtonPress(event) | Event::ButtonRelease(event) => event.event,
            Event::MotionNotify(event) => event.event,
            Event::FocusIn(event) | Event::FocusOut(event) => event.event,
            Event::LeaveNotify(event) => event.event,
            Event::Expose(event) => event.window,
            Event::ConfigureNotify(event) => event.window,
            Event::ClientMessage(event) => event.window,
            Event::Error(error) => return Err(format!("X11 protocol error: {error:?}").into()),
            _ => return Ok(()),
        };
        if window != self.surface.window {
            return Ok(());
        }
        if self.dialog.is_some()
            && matches!(
                event,
                Event::KeyPress(_)
                    | Event::KeyRelease(_)
                    | Event::ButtonPress(_)
                    | Event::ButtonRelease(_)
                    | Event::MotionNotify(_)
            )
        {
            return Ok(());
        }
        if let Event::KeyPress(key) | Event::KeyRelease(key) = &event
            && self.ime.forward(key)?
        {
            return Ok(());
        }
        let display = &self.surface.display;
        let translated = match event {
            Event::KeyPress(event) => {
                let key = self.keyboard.key(&event, true);
                if key.key.is_some_and(|value| {
                    crate::theme::is_toggle(crate::keys::Chord {
                        key: value,
                        mods: key.mods,
                    })
                }) {
                    if let Err(error) = crate::theme::toggle() {
                        log::error!("【X11普通窗口】【主题切换】{error}");
                    }
                    self.refresh_theme();
                    None
                } else {
                    Some(WindowEvent::Key(key))
                }
            }
            Event::KeyRelease(event) => {
                Some(WindowEvent::KeyRelease(self.keyboard.key(&event, false)))
            }
            Event::MotionNotify(event) => {
                self.pointer = (event.event_x.into(), event.event_y.into());
                self.drag_to((event.root_x, event.root_y))?;
                Some(WindowEvent::PointerMove {
                    position: self.pointer,
                })
            }
            Event::ButtonPress(event) | Event::ButtonRelease(event) => {
                let pressed = event.response_type == BUTTON_PRESS_EVENT;
                self.pointer = (event.event_x.into(), event.event_y.into());
                if !pressed {
                    self.drag = None;
                }
                if pressed && (4..=7).contains(&event.detail) {
                    let angle = match event.detail {
                        4 => (0.0, 120.0),
                        5 => (0.0, -120.0),
                        6 => (120.0, 0.0),
                        _ => (-120.0, 0.0),
                    };
                    let action = self.content.scroll(ScrollInput {
                        position: self.pointer,
                        angle,
                        delta: (-angle.0 / 12.0, -angle.1 / 12.0),
                        source: ScrollSource::Wheel,
                        mods: input::modifiers(event.state),
                    });
                    self.action(action);
                    None
                } else if event.detail <= 3 {
                    Some(WindowEvent::PointerButton {
                        position: self.pointer,
                        button: match event.detail {
                            1 => LEFT_BUTTON,
                            3 => RIGHT_BUTTON,
                            _ => 0x112,
                        },
                        pressed,
                    })
                } else {
                    None
                }
            }
            Event::FocusIn(event) if event.mode == NotifyMode::NORMAL => {
                self.text_focus(true)?;
                Some(WindowEvent::Focused(true))
            }
            Event::FocusOut(event) if event.mode == NotifyMode::NORMAL => {
                self.keyboard.reset();
                self.text_focus(false)?;
                Some(WindowEvent::Focused(false))
            }
            Event::LeaveNotify(_) => Some(WindowEvent::PointerLeave),
            Event::Expose(_) => {
                self.surface.repaint()?;
                None
            }
            Event::ConfigureNotify(event) => {
                self.sync_position()?;
                if self.surface.size != (event.width, event.height) {
                    self.surface.size = (event.width, event.height);
                    self.dirty = true;
                    Some(WindowEvent::Resize {
                        size: (event.width.into(), event.height.into()),
                    })
                } else {
                    None
                }
            }
            Event::ClientMessage(event)
                if event.type_ == display.atom("WM_PROTOCOLS")?
                    && event.data.as_data32()[0] == display.atom("WM_DELETE_WINDOW")? =>
            {
                if self.dialog.is_none() && self.content.close_requested() {
                    self.close = true;
                }
                None
            }
            _ => None,
        };
        if let Some(event) = translated {
            if self.dialog.is_some() && event.is_interaction() {
                return Ok(());
            }
            let action = self.content.event(event);
            self.action(action);
        }
        self.sync_text_input()?;
        Ok(())
    }
}
