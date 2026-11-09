//! 【X11普通窗口】【原生菜单】用独立窗口承载菜单像素并转换局部输入

use super::*;
use crate::{backend::x11::input, keys::Chord, types::popup::PopupEvent};

impl Node {
    /// 【X11菜单】【表面同步】无参数，同步当前内容请求的菜单链及变化像素，返回协议错误
    pub(super) fn sync_popups(&mut self) -> crate::AppResult {
        let nodes = self.content.popup_nodes();
        if !crate::types::popup_tree::valid(&nodes, self.content.popup_id()) {
            return Err("invalid X11 popup chain".into());
        }
        self.popups
            .retain(|(id, _)| nodes.iter().any(|node| node.id == *id));
        if nodes.is_empty() {
            self.popup_grab = None;
        }
        for node in nodes {
            let id = node.id;
            let Some(frame) = self.content.popup_frame_for(id, 1.0) else {
                continue;
            };
            crate::types::popup::validate(&frame)?;
            let display = self.surface.display.clone();
            let origin = node
                .parent
                .and_then(|parent| {
                    self.popups
                        .iter()
                        .find(|(id, _)| *id == parent)
                        .map(|(_, surface)| surface.position)
                })
                .unwrap_or(self.surface.position);
            let offset = match node.placement {
                crate::types::popup_tree::Placement::Point => frame.anchor,
                crate::types::popup_tree::Placement::BelowControl(rect) => {
                    (rect.left(), rect.bottom() + 4.0)
                }
                crate::types::popup_tree::Placement::Submenu {
                    parent_width,
                    row_top,
                    item_inset,
                } => (parent_width - 2.0, row_top - item_inset),
            };
            let position = (
                (i32::from(origin.0) + offset.0.round() as i32).clamp(
                    0,
                    (i32::from(display.screen().width_in_pixels) - frame.pixel_size.0 as i32)
                        .max(0),
                ),
                (i32::from(origin.1) + offset.1.round() as i32).clamp(
                    0,
                    (i32::from(display.screen().height_in_pixels) - frame.pixel_size.1 as i32)
                        .max(0),
                ),
            );
            let index = if let Some(index) = self.popups.iter().position(|(value, _)| *value == id)
            {
                index
            } else {
                self.popups.push((
                    id,
                    Surface::new(display.clone(), position, frame.pixel_size, true)?,
                ));
                self.popups.len() - 1
            };
            self.popups[index].1.present(frame.pixels)?;
        }
        if !self.popups.is_empty() && self.popup_grab.is_none() {
            self.popup_grab = Some(super::super::popup_grab::Grab::new(
                self.surface.display.clone(),
                self.surface.window,
            )?);
        }
        Ok(())
    }

    /// 【X11菜单】【输入路由】按event处理菜单内事件及外部关闭，返回事件是否由菜单消费
    pub(super) fn popup_event(&mut self, event: &Event) -> crate::AppResult<bool> {
        if self.popups.is_empty() {
            return Ok(false);
        }
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
                } else {
                    return Ok(false);
                }
            }
            Event::KeyPress(event) => {
                let input = self.keyboard.key(event, true);
                let (id, _) = self.popups.last().unwrap();
                let action = self.content.popup_event(
                    *id,
                    PopupEvent::Key {
                        chord: input.key.map(|key| Chord {
                            key,
                            mods: input.mods,
                        }),
                        text: (!input.text.is_empty()).then_some(input.text),
                    },
                );
                self.popup_action(*id, action);
                return Ok(true);
            }
            Event::Expose(event) => {
                if let Some((_, surface)) = self
                    .popups
                    .iter()
                    .find(|(_, surface)| surface.window == event.window)
                {
                    surface.repaint()?;
                    return Ok(true);
                }
                return Ok(false);
            }
            _ => return Ok(false),
        };
        if let Some((id, _)) = self
            .popups
            .iter()
            .find(|(_, surface)| surface.window == window)
        {
            let id = *id;
            let action = self.content.popup_event(id, translated);
            self.popup_action(id, action);
            return Ok(true);
        }
        if matches!(event, Event::ButtonPress(_)) {
            if let Some((id, _)) = self.popups.first() {
                let id = *id;
                self.popups.clear();
                self.popup_grab = None;
                let action = self.content.popup_closed(id, None);
                self.action(action);
            }
            return Ok(true);
        }
        Ok(false)
    }

    /// 【X11菜单】【动作交付】按id和action先销毁菜单再提交选择，避免已关闭菜单接收旧事件
    fn popup_action(&mut self, id: u64, action: WindowPopupAction) {
        match action {
            WindowPopupAction::None => {}
            WindowPopupAction::Redraw => self.dirty = true,
            WindowPopupAction::Close
            | WindowPopupAction::CloseAll
            | WindowPopupAction::Activate(_) => {
                self.popups.clear();
                self.popup_grab = None;
                let selected = if let WindowPopupAction::Activate(index) = action {
                    Some(index)
                } else {
                    None
                };
                let action = self.content.popup_closed(id, selected);
                self.action(action);
            }
        }
    }
}
