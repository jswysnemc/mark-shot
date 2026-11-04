//! 【原生窗口】【弹层输入】表面身份隔离菜单和父窗口，右键及外部点击不穿透编辑器

use super::{WindowEvent, runtime::NativeWindow, surface::SurfaceId};
use crate::types::{MouseButton, popup::PopupEvent};
use smithay_client_toolkit::seat::{
    keyboard::KeyEvent,
    pointer::{PointerEvent, PointerEventKind},
};
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_protocols::wp::cursor_shape::v1::client::wp_cursor_shape_device_v1::Shape;

impl NativeWindow {
    /// 【原生窗口】【菜单表面】返回 surface 是否为当前存活弹层，比较协议对象而非可复用数字编号
    pub fn is_popup_surface(&self, surface: &WlSurface) -> bool {
        self.popup
            .as_ref()
            .is_some_and(|popup| popup.popup.wl_surface() == surface)
    }

    /// 【原生窗口】【菜单指针】转换 event 的菜单本地坐标，返回是否消费；父窗口按下只关闭菜单，禁止穿透
    pub fn popup_pointer(&mut self, event: &PointerEvent) -> bool {
        let Some(popup) = &self.popup else {
            return false;
        };
        let id = popup.id;
        if popup.popup.wl_surface() != &event.surface {
            if self.main.matches(&event.surface)
                && matches!(event.kind, PointerEventKind::Press { .. })
            {
                self.finish_popup(id, None);
            }
            return true;
        }
        let (x, y) = event.position;
        let input = match event.kind {
            PointerEventKind::Enter { serial } => {
                self.enter_serial = serial;
                if let Some(cursor) = &self.cursor {
                    cursor.set_shape(serial, Shape::Default);
                }
                PopupEvent::Pointer { x, y }
            }
            PointerEventKind::Motion { .. } => PopupEvent::Pointer { x, y },
            PointerEventKind::Leave { .. } => PopupEvent::Leave,
            PointerEventKind::Press { button, .. } | PointerEventKind::Release { button, .. } => {
                let button = match button {
                    0x110 => MouseButton::Left,
                    0x111 => MouseButton::Right,
                    0x112 => MouseButton::Middle,
                    _ => return true,
                };
                PopupEvent::Button {
                    x,
                    y,
                    button,
                    pressed: matches!(event.kind, PointerEventKind::Press { .. }),
                }
            }
            PointerEventKind::Axis { .. } => return true,
        };
        self.popup_dispatch(id, input);
        true
    }

    /// 【原生窗口】【菜单按键】菜单获得焦点或父窗口暂时仍拥有焦点时消费 event，返回是否分派；快捷键不能穿透正文
    pub fn popup_key(&mut self, event: &KeyEvent) -> bool {
        let Some(popup) = &self.popup else {
            return false;
        };
        if !popup.keyboard_focused && self.keyboard_surface != Some(SurfaceId::Main) {
            return false;
        }
        let chord =
            crate::backend::wayland::keysym::chord(event.keysym, event.raw_code, self.modifiers);
        let id = popup.id;
        self.popup_dispatch(
            id,
            PopupEvent::Key {
                chord,
                text: event.utf8.clone(),
            },
        );
        true
    }

    /// 【原生窗口】【菜单焦点进入】surface 为菜单时记录独立焦点并返回真，不把父内容误判为重新获得编辑资格
    pub fn popup_enter(&mut self, surface: &WlSurface) -> bool {
        if !self.is_popup_surface(surface) {
            return false;
        }
        self.popup.as_mut().unwrap().keyboard_focused = true;
        true
    }

    /// 【原生窗口】【菜单焦点离开】surface 为菜单时取消抓取并清理父窗口焦点，返回是否消费；过期表面不影响新输入
    pub fn popup_leave(&mut self, surface: &WlSurface) -> bool {
        if !self.is_popup_surface(surface) {
            return false;
        }
        let id = self.popup.as_ref().unwrap().id;
        self.finish_popup(id, None);
        self.keyboard_surface = None;
        self.dispatch(WindowEvent::Focused(false), None);
        true
    }
}
