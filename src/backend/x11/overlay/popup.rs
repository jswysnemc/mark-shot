//! 【X11覆盖菜单】【原生表面】编辑器菜单独立呈现，关闭后不再交付旧编号输入

use super::*;
use crate::{
    backend::x11::input,
    keys::Chord,
    types::popup::{PopupEvent, PopupFrame},
};
use x11rb::protocol::{Event, xproto::*};

impl Overlay {
    /// 【X11覆盖菜单】【像素同步】按frame创建或更新菜单，空值释放表面，返回支持状态及协议错误
    pub(super) fn popup_frame(&mut self, frame: Option<PopupFrame<'_>>) -> crate::AppResult<bool> {
        let Some(frame) = frame else {
            self.popup = None;
            self.popup_grab = None;
            return Ok(true);
        };
        crate::types::popup::validate(&frame)?;
        let parent = &self
            .surfaces
            .get(frame.monitor_idx)
            .ok_or("X11 popup output missing")?
            .1;
        let left = (i32::from(parent.position.0) + frame.anchor.0.round() as i32).clamp(
            0,
            (i32::from(self.display.screen().width_in_pixels) - frame.pixel_size.0 as i32).max(0),
        );
        let top = (i32::from(parent.position.1) + frame.anchor.1.round() as i32).clamp(
            0,
            (i32::from(self.display.screen().height_in_pixels) - frame.pixel_size.1 as i32).max(0),
        );
        if !self.popup.as_ref().is_some_and(|(id, surface)| {
            *id == frame.id
                && surface.size == (frame.pixel_size.0 as u16, frame.pixel_size.1 as u16)
        }) {
            let surface = Surface::new(self.display.clone(), (left, top), frame.pixel_size, true)?;
            self.popup = Some((frame.id, surface));
            self.pending.push_back(OverlayEvent::Popup {
                id: frame.id,
                event: PopupEvent::Configured {
                    position: (
                        left - i32::from(parent.position.0),
                        top - i32::from(parent.position.1),
                    ),
                },
            });
        }
        self.popup.as_mut().unwrap().1.present(frame.pixels)?;
        if self.popup_grab.is_none() {
            self.popup_grab = Some(super::super::popup_grab::Grab::new(
                self.display.clone(),
                parent.window,
            )?);
        }
        Ok(true)
    }

    /// 【X11覆盖菜单】【事件路由】按event把菜单输入排入共享事件队列，返回是否消费
    pub(super) fn popup_input(&mut self, event: &Event) -> crate::AppResult<bool> {
        let Some((id, surface)) = &self.popup else {
            return Ok(false);
        };
        let id = *id;
        let (window, translated) = match event {
            Event::MotionNotify(event) => (
                event.event,
                PopupEvent::Pointer {
                    x: event.event_x.into(),
                    y: event.event_y.into(),
                },
            ),
            Event::LeaveNotify(event) => (event.event, PopupEvent::Leave),
            Event::ButtonPress(event) | Event::ButtonRelease(event) => {
                if let Some(button) = input::button(event.detail) {
                    (
                        event.event,
                        PopupEvent::Button {
                            x: event.event_x.into(),
                            y: event.event_y.into(),
                            button,
                            pressed: event.response_type == BUTTON_PRESS_EVENT,
                        },
                    )
                } else if event.response_type == BUTTON_PRESS_EVENT && matches!(event.detail, 4 | 5)
                {
                    (
                        event.event,
                        PopupEvent::Scroll {
                            delta: (0.0, if event.detail == 4 { 10.0 } else { -10.0 }),
                        },
                    )
                } else {
                    return Ok(false);
                }
            }
            Event::KeyPress(event) => {
                let key = self.keyboard.key(event, true);
                (
                    surface.window,
                    PopupEvent::Key {
                        chord: key.key.map(|value| Chord {
                            key: value,
                            mods: key.mods,
                        }),
                        text: (!key.text.is_empty()).then_some(key.text),
                    },
                )
            }
            Event::Expose(event) if event.window == surface.window => {
                surface.repaint()?;
                return Ok(true);
            }
            _ => return Ok(false),
        };
        if window == surface.window {
            self.pending.push_back(OverlayEvent::Popup {
                id,
                event: translated,
            });
            return Ok(true);
        }
        if matches!(event, Event::ButtonPress(_)) {
            self.popup = None;
            self.popup_grab = None;
            self.pending.push_back(OverlayEvent::Popup {
                id,
                event: PopupEvent::Closed,
            });
            return Ok(true);
        }
        Ok(false)
    }
}
