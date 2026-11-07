//! 【Windows覆盖层】【事件分派】原生窗口编号与输出索引分别校验，过期事件不进入编辑业务

use super::*;
use crate::{
    backend::window::WindowEvent,
    keys::Chord,
    types::{
        MouseButton,
        popup::PopupEvent,
        scroll::{ScrollAxis, ScrollEvent, ScrollSource},
    },
};
use std::time::{Duration, Instant};

/// 【Windows覆盖层】【鼠标按钮】按原生button返回共享鼠标职责，其他按钮保持未处理
fn button(button: u32) -> Option<MouseButton> {
    match button {
        0x110 => Some(MouseButton::Left),
        0x111 => Some(MouseButton::Right),
        0x112 => Some(MouseButton::Middle),
        _ => None,
    }
}

impl Overlay {
    /// 【Windows覆盖层】【输入归属】按event收集输出或弹层输入，无返回值，只向存活窗口交付一次
    fn collect(&mut self, event: native::Event) {
        let id = event.id();
        let popup = self
            .popup
            .as_ref()
            .filter(|entry| entry.2.id() == id)
            .map(|entry| entry.0);
        let monitor = self.surfaces.iter().position(|entry| entry.1.id() == id);
        if popup.is_none() && monitor.is_none() {
            return;
        }
        if let native::Event::Key { modifiers, .. } = &event {
            self.pending.push_back(OverlayEvent::ModifiersChanged {
                ctrl: modifiers.ctrl,
                shift: modifiers.shift,
            });
        }
        if matches!(event, native::Event::Close { .. }) {
            self.pending.push_back(OverlayEvent::EscapePressed);
            return;
        }
        if let native::Event::Composition {
            target,
            preedit,
            cursor,
            commit,
            ..
        } = &event
        {
            if self
                .text
                .as_ref()
                .is_some_and(|snapshot| snapshot.target_id == *target)
            {
                self.pending
                    .push_back(OverlayEvent::TextInput(crate::types::text_input::Update {
                        target_id: *target,
                        serial: 0,
                        sync_allowed: true,
                        batch: crate::types::text_input::Batch {
                            preedit: crate::types::text_input::Preedit {
                                text: preedit.clone(),
                                cursor: Some((*cursor, *cursor)),
                            },
                            commit: commit.clone(),
                            ..Default::default()
                        },
                    }));
            }
            return;
        }
        let timestamp = if let native::Event::Button { time, .. } = event {
            Some(time)
        } else {
            None
        };
        let Some(event) = self.decoder.event(event, 1.0) else {
            return;
        };
        if let Some(id) = popup {
            let event = match event {
                WindowEvent::PointerMove { position } => PopupEvent::Pointer {
                    x: position.0,
                    y: position.1,
                },
                WindowEvent::PointerButton {
                    position,
                    button: code,
                    pressed,
                } => {
                    let Some(button) = button(code) else {
                        return;
                    };
                    PopupEvent::Button {
                        x: position.0,
                        y: position.1,
                        button,
                        pressed,
                    }
                }
                WindowEvent::PointerLeave => PopupEvent::Leave,
                WindowEvent::Wheel(input) => PopupEvent::Scroll {
                    delta: (-input.delta.0, -input.delta.1),
                },
                _ => return,
            };
            self.pending.push_back(OverlayEvent::Popup { id, event });
            return;
        }
        let monitor_idx = monitor.unwrap();
        match event {
            WindowEvent::PointerMove { position } => {
                self.cursor_output = Some(monitor_idx);
                self.pending.push_back(OverlayEvent::PointerMove {
                    monitor_idx,
                    x: position.0,
                    y: position.1,
                    history: Vec::new(),
                });
            }
            WindowEvent::Focused(true) => {
                let focus = crate::types::text_input::Focus {
                    id: crate::features::text::input_method::focus::identity(),
                    monitor_idx,
                };
                self.focus = Some(focus);
                self.pending.push_back(OverlayEvent::Focus { monitor_idx });
                self.pending.push_back(OverlayEvent::TextInputFocus(focus));
            }
            WindowEvent::Focused(false) => {
                if let Some(focus) = self.focus.take() {
                    self.pending
                        .push_back(OverlayEvent::TextInputLeave { focus_id: focus.id });
                }
                self.pending
                    .push_back(OverlayEvent::FocusLost { monitor_idx });
            }
            WindowEvent::PointerButton {
                position,
                button: code,
                pressed,
            } => {
                if self.popup.is_some() {
                    let id = self.popup.take().unwrap().0;
                    self.pending.push_back(OverlayEvent::Popup {
                        id,
                        event: PopupEvent::Closed,
                    });
                    return;
                }
                let Some(button) = button(code) else {
                    return;
                };
                self.pending.push_back(OverlayEvent::PointerMove {
                    monitor_idx,
                    x: position.0,
                    y: position.1,
                    history: Vec::new(),
                });
                self.pending.push_back(OverlayEvent::PointerButton {
                    button,
                    pressed,
                    serial: None,
                    timestamp,
                });
            }
            WindowEvent::Key(input) => {
                let chord = input.key.map(|key| Chord {
                    key,
                    mods: input.mods,
                });
                let text = (!input.text.is_empty()).then_some(input.text);
                if let Some((id, _, _)) = self.popup.as_ref() {
                    self.pending.push_back(OverlayEvent::Popup {
                        id: *id,
                        event: PopupEvent::Key { chord, text },
                    });
                } else {
                    self.pending.push_back(OverlayEvent::Key { chord, text });
                }
            }
            WindowEvent::Wheel(input) => {
                self.pending.push_back(OverlayEvent::ModifiersChanged {
                    ctrl: input.mods.ctrl,
                    shift: input.mods.shift,
                });
                let axis = |angle| ScrollAxis {
                    value120: angle as i32,
                    ..Default::default()
                };
                self.pending.push_back(OverlayEvent::Scroll(ScrollEvent {
                    horizontal: axis(input.angle.0),
                    vertical: axis(input.angle.1),
                    source: Some(ScrollSource::Wheel),
                }));
            }
            _ => {}
        }
    }

    /// 【Windows覆盖层】【阻塞事件】按timeout等待已有来源，返回下一个共享事件，无效消息不会重置调用方期限
    pub(super) fn wait_event(
        &mut self,
        timeout: Option<Duration>,
    ) -> crate::AppResult<OverlayEvent> {
        let deadline = timeout.map(|duration| Instant::now() + duration);
        loop {
            while let Some(event) = self.queue.pop() {
                self.collect(event);
            }
            if let Some(event) = self.pending.pop_front() {
                return Ok(event);
            }
            if let Some(watch) = &self.theme {
                match watch.changed() {
                    Ok(Some(mode)) => {
                        crate::theme::apply_saved(mode);
                        return Ok(OverlayEvent::ThemeChanged);
                    }
                    Ok(None) => {}
                    Err(error) => log::error!("【Windows覆盖层】【主题通知】{error}"),
                }
            }
            let mut handles = vec![self.wake.handle()];
            if let Some(watch) = &self.theme {
                handles.push(watch.handle());
            }
            let left = deadline.map(|deadline| deadline.saturating_duration_since(Instant::now()));
            match native::dispatch(&handles, left)? {
                native::Wait::Source(0) | native::Wait::Timeout => return Ok(OverlayEvent::Tick),
                _ => {}
            }
        }
    }
}
