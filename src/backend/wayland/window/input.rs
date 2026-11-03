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

use super::{KeyInput, WindowCursor, WindowEvent, runtime::NativeWindow};
use crate::{backend::wayland::keysym, keys::Mods};

impl NativeWindow {
    /// 【原生窗口】【按键转换】将 event 的键名、文字与修饰键发送给内容；不替内容执行快捷键
    fn key_event(&mut self, event: KeyEvent) {
        let key =
            keysym::chord(event.keysym, event.raw_code, self.modifiers).map(|chord| chord.key);
        self.dispatch(
            WindowEvent::Key(KeyInput {
                key,
                text: event.utf8.unwrap_or_default(),
                mods: self.modifiers,
            }),
            None,
        );
    }

    /// 【原生窗口】【鼠标形状】按 position 对应内容更新指针；无形状协议时保持合成器默认值
    fn update_cursor(&self, position: (f64, f64)) {
        if let Some(cursor) = &self.cursor {
            let shape = match self.content.cursor(position) {
                WindowCursor::Default => Shape::Default,
                WindowCursor::Text => Shape::Text,
                WindowCursor::Resize => Shape::SeResize,
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
        _: &QueueHandle<Self>,
        _: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        for event in events {
            if &event.surface != self.window.wl_surface() {
                continue;
            }
            let position = event.position;
            match event.kind {
                PointerEventKind::Enter { serial } => {
                    self.enter_serial = serial;
                    self.update_cursor(position);
                    self.dispatch(WindowEvent::PointerMove { position }, None);
                }
                PointerEventKind::Motion { .. } => {
                    self.update_cursor(position);
                    self.dispatch(WindowEvent::PointerMove { position }, None);
                }
                PointerEventKind::Press { button, serial, .. } => {
                    self.dispatch(
                        WindowEvent::PointerButton {
                            position,
                            button,
                            pressed: true,
                        },
                        Some(serial),
                    );
                }
                PointerEventKind::Release { button, .. } => {
                    self.dispatch(
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
                    self.dispatch(
                        WindowEvent::Scroll {
                            position,
                            delta: (horizontal.absolute, vertical.absolute),
                        },
                        None,
                    );
                }
                PointerEventKind::Leave { .. } => self.dispatch(WindowEvent::PointerLeave, None),
            }
        }
    }
}

use smithay_client_toolkit::shell::WaylandSurface;

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
        _: &wl_surface::WlSurface,
        _: u32,
        _: &[u32],
        _: &[Keysym],
    ) {
        self.dispatch(WindowEvent::Focused(true), None);
    }

    /// 【原生窗口】【失去焦点】清除修饰键并通知内容；保留编辑历史
    fn leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: &wl_surface::WlSurface,
        _: u32,
    ) {
        self.modifiers = Mods::default();
        self.dispatch(WindowEvent::Focused(false), None);
    }

    /// 【原生窗口】【按键释放】文字编辑只处理按下事件；释放不修改内容
    fn release_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        _: KeyEvent,
    ) {
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
                if let Some(keyboard) = self.keyboard.take()
                    && keyboard.version() >= 3
                {
                    keyboard.release();
                }
                self.keyboard_seat = None;
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
