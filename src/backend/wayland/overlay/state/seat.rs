// ── Seat Management ──────────────────────────────────────────────────────────
// reminder: wl_seat represents a logical input device that bundles together
// stuff like mouse (pointer), keyboard
//
// We monitor for new capabilities (keyboard & mouse) to initialize input handlers
use smithay_client_toolkit::seat::{Capability, SeatHandler};
use wayland_client::protocol::wl_seat;
use wayland_client::{Connection, QueueHandle};

use crate::backend::wayland::overlay::state::OverlayState;

impl SeatHandler for OverlayState {
    fn seat_state(&mut self) -> &mut smithay_client_toolkit::seat::SeatState {
        &mut self.seat
    }
    /// 【Wayland输入】【设备绑定】按 seat 的 capability 通过 qh 创建输入设备，完成后刷新连接；空闲时也能收到键盘映射，无返回值
    fn new_capability(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: smithay_client_toolkit::seat::Capability,
    ) {
        match capability {
            Capability::Pointer => {
                // Initialize pointer and bind the cursor shape manager
                // to allow controlling cursors shape on our overlay
                let Ok(pointer) = self.seat.get_pointer(qh, &seat) else {
                    return;
                };
                let device = self.cursor_shape_manager.get_shape_device(&pointer, qh);
                self.cursor_shape_device = Some(device);
            }
            Capability::Keyboard => {
                // Initialize keyboard
                let _ = self.seat.get_keyboard(qh, &seat, None);
                self.text_input.bind(&seat, qh);
            }
            _ => {}
        }
        // 1. 【Wayland输入】【绑定交付】能力通知本身不产生画布事件，不能等待下一次重绘才提交设备请求
        self.pending_flush = true;
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
    /// 【输入法】【能力移除】按 seat、capability 移除键盘输入法对象并通知失焦，其他参数来自协议，无返回值
    fn remove_capability(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: smithay_client_toolkit::seat::Capability,
    ) {
        if capability == Capability::Keyboard {
            if let Some(event) = self.text_input.remove(&seat) {
                self.events.push_back(event);
            }
            self.pending_flush = true;
        }
    }
    /// 【输入法】【设备释放】移除 seat 的输入法对象并交付失焦，其他参数来自协议，无返回值
    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, seat: wl_seat::WlSeat) {
        if let Some(event) = self.text_input.remove(&seat) {
            self.events.push_back(event);
        }
        self.pending_flush = true;
    }
}
