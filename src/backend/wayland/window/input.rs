//! 【原生窗口】【输入分发】将 Wayland 输入转换为内容事件并维护设备生命周期

use smithay_client_toolkit::seat::{
    Capability, SeatHandler, SeatState,
    keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers, RepeatInfo},
    pointer::{PointerEvent, PointerEventKind, PointerHandler},
};
use wayland_client::{
    Connection, Proxy, QueueHandle,
    protocol::{wl_keyboard, wl_pointer, wl_seat, wl_surface},
};
use wayland_protocols::wp::cursor_shape::v1::client::wp_cursor_shape_device_v1::Shape;

use super::surface::SurfaceId;
use super::{KeyInput, WindowCursor, WindowEvent, runtime::NativeWindow};
use crate::{backend::wayland::keysym, keys::Mods};

impl NativeWindow {
    /// 【原生窗口】【按键转换】将 event 的键名、文字与修饰键发送给内容；不替内容执行快捷键
    fn key_event(&mut self, event: KeyEvent) {
        if self.popup_key(&event) {
            return;
        }
        let Some(id) = self.keyboard_surface else {
            return;
        };
        let key =
            keysym::chord(event.keysym, event.raw_code, self.modifiers).map(|chord| chord.key);
        self.dispatch_to(
            id,
            WindowEvent::Key(KeyInput {
                key,
                text: event.utf8.unwrap_or_default(),
                mods: self.modifiers,
            }),
            None,
        );
    }

    /// 【原生窗口】【鼠标形状】按 position 对应内容更新指针；无形状协议时保持合成器默认值
    fn update_cursor(&self, id: SurfaceId, position: (f64, f64)) {
        if let Some(cursor) = &self.cursor {
            let kind = if id == SurfaceId::Main && self.dialog.is_some() {
                WindowCursor::Default
            } else {
                self.content_for(id)
                    .map_or(WindowCursor::Default, |content| content.cursor(position))
            };
            let shape = match kind {
                WindowCursor::Default => Shape::Default,
                WindowCursor::Text => Shape::Text,
                WindowCursor::Pointer => Shape::Pointer,
                WindowCursor::Resize => Shape::SeResize,
                WindowCursor::HorizontalResize => Shape::ColResize,
            };
            cursor.set_shape(self.enter_serial, shape);
        }
    }
}

impl PointerHandler for NativeWindow {
    /// 【原生窗口】【鼠标输入】按 events 顺序分发窗口内的移动、按键与滚动；移动请求使用真实序列号
    fn pointer_frame(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        _: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        for event in events {
            if self.popup_pointer(event) {
                continue;
            }
            let Some(id) = self.surface_id(&event.surface) else {
                continue;
            };
            let position = event.position;
            match event.kind {
                PointerEventKind::Enter { serial } => {
                    self.enter_serial = serial;
                    self.update_cursor(id, position);
                    self.dispatch_to(id, WindowEvent::PointerMove { position }, None);
                }
                PointerEventKind::Motion { .. } => {
                    self.update_cursor(id, position);
                    self.dispatch_to(id, WindowEvent::PointerMove { position }, None);
                }
                PointerEventKind::Press { button, serial, .. } => {
                    self.dispatch_to(
                        id,
                        WindowEvent::PointerButton {
                            position,
                            button,
                            pressed: true,
                        },
                        Some(serial),
                    );
                    if id == SurfaceId::Main {
                        let seat = self.pointer_seat.clone();
                        self.sync_popup(qh, seat.as_ref().map(|seat| (seat, serial)));
                    }
                }
                PointerEventKind::Release { button, .. } => {
                    self.dispatch_to(
                        id,
                        WindowEvent::PointerButton {
                            position,
                            button,
                            pressed: false,
                        },
                        None,
                    );
                }
                PointerEventKind::Axis {
                    horizontal,
                    vertical,
                    ..
                } => {
                    self.dispatch_to(
                        id,
                        WindowEvent::Scroll {
                            position,
                            delta: (horizontal.absolute, vertical.absolute),
                        },
                        None,
                    );
                }
                PointerEventKind::Leave { .. } => {
                    self.dispatch_to(id, WindowEvent::PointerLeave, None)
                }
            }
        }
    }
}

impl KeyboardHandler for NativeWindow {
    /// 【原生窗口】【按键按下】转换 event 并交给内容处理
    fn press_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        self.key_event(event);
    }

    /// 【原生窗口】【重复按键】重复输入采用与按下相同的转换规则
    fn repeat_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        self.key_event(event);
    }

    /// 【原生窗口】【修饰键】记录 modifiers，供后续按键与快捷键判断使用
    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        modifiers: Modifiers,
        _: RawModifiers,
        _: u32,
    ) {
        self.modifiers = keysym::mods(&modifiers);
    }

    /// 【原生窗口】【获得焦点】通知内容获得键盘焦点；不重置文字或选择
    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        surface: &wl_surface::WlSurface,
        _: u32,
        _: &[u32],
        _: &[Keysym],
    ) {
        if self.popup_enter(surface) {
            return;
        }
        let Some(id) = self.surface_id(surface) else {
            return;
        };
        self.keyboard_surface = Some(id);
        self.dispatch_to(id, WindowEvent::Focused(true), None);
    }

    /// 【原生窗口】【失去焦点】清除修饰键并通知内容；保留编辑历史
    fn leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        surface: &wl_surface::WlSurface,
        _: u32,
    ) {
        self.modifiers = Mods::default();
        if self.popup_leave(surface) {
            return;
        }
        if let Some(id) = self.surface_id(surface) {
            if id == SurfaceId::Main && self.popup.is_some() {
                return;
            }
            if self.keyboard_surface == Some(id) {
                self.keyboard_surface = None;
            }
            self.dispatch_to(id, WindowEvent::Focused(false), None);
        }
    }

    /// 【原生窗口】【按键释放】把 event 的释放交给内容，文字编辑默认忽略，按钮可在释放时执行，无返回值
    fn release_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        if self.popup.is_some() {
            return;
        }
        let Some(id) = self.keyboard_surface else {
            return;
        };
        let key =
            keysym::chord(event.keysym, event.raw_code, self.modifiers).map(|chord| chord.key);
        self.dispatch_to(
            id,
            WindowEvent::KeyRelease(KeyInput {
                key,
                text: String::new(),
                mods: self.modifiers,
            }),
            None,
        );
    }

    /// 【原生窗口】【重复配置】重复时序由输入设备与工具包负责；不增加空闲定时器
    fn update_repeat_info(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: RepeatInfo,
    ) {
    }
}

impl SeatHandler for NativeWindow {
    /// 【原生窗口】【输入座席】返回当前连接维护的座席状态
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seats
    }

    /// 【原生窗口】【设备绑定】为 seat 的 capability 创建一个指针或键盘；重复能力通知不重复绑定
    fn new_capability(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        match capability {
            Capability::Pointer if self.pointer.is_none() => {
                if let Ok(pointer) = self.seats.get_pointer(qh, &seat) {
                    self.cursor = self
                        .cursor_shapes
                        .as_ref()
                        .map(|manager| manager.get_shape_device(&pointer, qh));
                    self.pointer = Some(pointer);
                    self.pointer_seat = Some(seat);
                }
            }
            Capability::Keyboard if self.keyboard.is_none() => {
                if let Ok(keyboard) = self.seats.get_keyboard_with_repeat(
                    qh,
                    &seat,
                    None,
                    self.loop_handle.clone(),
                    Box::new(|state, _, event| state.key_event(event)),
                ) {
                    self.text_input.bind(&seat, qh);
                    self.keyboard = Some(keyboard);
                    self.keyboard_seat = Some(seat);
                }
            }
            _ => {}
        }
    }

    /// 【原生窗口】【新增座席】设备等待能力通知再按需绑定
    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    /// 【原生窗口】【移除设备】释放 seat 已移除的 capability 对象；不影响其他座席
    fn remove_capability(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        match capability {
            Capability::Pointer if self.pointer_seat.as_ref() == Some(&seat) => {
                if let Some(cursor) = self.cursor.take() {
                    cursor.destroy();
                }
                if let Some(pointer) = self.pointer.take()
                    && pointer.version() >= 3
                {
                    pointer.release();
                }
                self.pointer_seat = None;
            }
            Capability::Keyboard if self.keyboard_seat.as_ref() == Some(&seat) => {
                if let Some(focus) = self.text_input.remove(&seat) {
                    self.leave_text_input(focus.id);
                }
                if let Some(keyboard) = self.keyboard.take()
                    && keyboard.version() >= 3
                {
                    keyboard.release();
                }
                self.keyboard_seat = None;
                if let Some(id) = self.keyboard_surface.take() {
                    self.dispatch_to(id, WindowEvent::Focused(false), None);
                }
                self.modifiers = Mods::default();
            }
            _ => {}
        }
    }

    /// 【原生窗口】【移除座席】释放 seat 关联的全部输入设备
    fn remove_seat(&mut self, conn: &Connection, qh: &QueueHandle<Self>, seat: wl_seat::WlSeat) {
        self.remove_capability(conn, qh, seat.clone(), Capability::Pointer);
        self.remove_capability(conn, qh, seat, Capability::Keyboard);
    }
}
