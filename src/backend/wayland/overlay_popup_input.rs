//! 【截图弹层】【指针分流】原生菜单只接收自身表面输入，主画布与菜单不共享指针坐标

use crate::{
    backend::wayland::overlay::state::OverlayState,
    types::{CursorIcon, MouseButton, popup::PopupEvent},
};
use smithay_client_toolkit::seat::pointer::{PointerEvent, PointerEventKind};

impl OverlayState {
    /// 【截图弹层】【指针事件】按 event 判断当前弹层表面，转换菜单本地坐标并返回是否消费，不产生画布拖动
    pub fn popup_pointer(&mut self, event: &PointerEvent) -> bool {
        if !self
            .popup
            .as_ref()
            .is_some_and(|popup| popup.popup.wl_surface() == &event.surface)
        {
            return false;
        }
        let (x, y) = event.position;
        match event.kind {
            PointerEventKind::Enter { serial } => {
                self.pointer_enter_serial = serial;
                self.pointer_surface_idx = None;
                self.cursor_applied_since_enter = false;
                self.apply_cursor(CursorIcon::Default);
                self.popup_event(PopupEvent::Pointer { x, y });
            }
            PointerEventKind::Motion { .. } => self.popup_event(PopupEvent::Pointer { x, y }),
            PointerEventKind::Leave { .. } => self.popup_event(PopupEvent::Leave),
            PointerEventKind::Press { button, .. } | PointerEventKind::Release { button, .. } => {
                let button = match button {
                    0x110 => MouseButton::Left,
                    0x111 => MouseButton::Right,
                    0x112 => MouseButton::Middle,
                    _ => return true,
                };
                self.popup_event(PopupEvent::Button {
                    x,
                    y,
                    button,
                    pressed: matches!(event.kind, PointerEventKind::Press { .. }),
                });
            }
            PointerEventKind::Axis { .. } => {}
        }
        true
    }
}
