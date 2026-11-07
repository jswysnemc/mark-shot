//! 【贴图】【输入职责】左键拖动或选择文字，右键只打开菜单，滚轮只改变显示比例

use super::runtime::Pin;
use crate::{
    features::pinned_image::menu::PinAction,
    keys::{Key, Mods},
};
use smithay_client_toolkit::seat::{
    Capability, SeatHandler, SeatState,
    keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers, RepeatInfo},
    pointer::{PointerEvent, PointerEventKind, PointerHandler},
};
use wayland_client::{
    Connection, QueueHandle,
    protocol::{wl_keyboard, wl_pointer, wl_seat, wl_surface},
};
const LEFT: u32 = 0x110;
const RIGHT: u32 = 0x111;

impl Pin {
    /// 【贴图】【快捷键】转换 event 为菜单导航或贴图快捷键；不把菜单 Escape 传给主窗口
    fn key(&mut self, event: KeyEvent, qh: &QueueHandle<Self>) {
        let chord = crate::backend::wayland::keysym::chord(event.keysym, event.raw_code, self.mods);
        if chord.is_some_and(crate::theme::is_toggle) {
            if let Err(error) = crate::theme::toggle() {
                log::error!("【贴图】【主题切换】无法保存主题: {error}");
            } else {
                self.refresh_theme();
            }
            return;
        }
        let key = chord.map(|chord| chord.key);
        if self.menu_key(key, qh) {
            return;
        }
        match key {
            Some(Key::Escape) => self.exit = true,
            Some(Key::Char('c')) if self.mods.ctrl => {
                self.action(
                    if self.text_selection && self.text.has_selection() {
                        PinAction::CopySelectedText
                    } else {
                        PinAction::Copy
                    },
                    qh,
                );
            }
            Some(Key::Char('s')) if self.mods.ctrl => self.action(PinAction::SaveAs, qh),
            Some(Key::Char('v')) if self.mods.ctrl => {
                let result = crate::utils::spawn_self(&["--pin-clipboard"], &[]);
                self.report(result);
            }
            _ => {}
        }
    }
}

impl Pin {
    /// 【贴图】【主题刷新】无参数，刷新主窗口和存活菜单，保留图像、菜单身份与译文选择，无返回值
    pub(super) fn refresh_theme(&mut self) {
        self.dirty = true;
        if self.popup.is_some() {
            let result = self.draw_menu();
            self.report(result);
        }
    }
}

impl PointerHandler for Pin {
    /// 【贴图】【指针事件】按 events 顺序区分主表面和菜单表面；右键不能选择菜单动作
    fn pointer_frame(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        _: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        for event in events {
            let position = event.position;
            if &event.surface == self.shell.surface() {
                self.last_position = position;
            }
            if self.menu_pointer(event, qh) {
                continue;
            }
            if &event.surface != self.shell.surface() {
                continue;
            }
            match event.kind {
                PointerEventKind::Enter { serial } => {
                    self.serial = serial;
                    // 1. 【贴图】【系统移动结束】合成器交还普通窗口指针后清除移动状态，原生缩放仍等待最终尺寸配置
                    if matches!(self.shell, super::runtime::Shell::Window(_))
                        && self
                            .drag
                            .is_some_and(|drag| matches!(drag.kind, super::gesture::Kind::Move))
                    {
                        self.finish_gesture(qh);
                    }
                    self.cursor(position);
                }
                PointerEventKind::Press {
                    button: RIGHT,
                    serial,
                    ..
                } => {
                    let result = self.open_menu(qh, position, serial);
                    self.report(result);
                }
                PointerEventKind::Press {
                    button: LEFT,
                    serial,
                    ..
                } => {
                    self.begin_gesture(position, serial);
                }
                PointerEventKind::Motion { .. } => self.continue_gesture(position, qh),
                PointerEventKind::Release { button: LEFT, .. } => {
                    self.finish_gesture(qh);
                    self.cursor(position);
                }
                PointerEventKind::Axis { vertical, .. } if self.popup.is_none() => {
                    let old_size = self.size;
                    let point = self.local_point(position);
                    let factor = 1.08_f32.powf(-vertical.absolute as f32 / 15.0);
                    if self.image.scale(self.image.zoom * factor) {
                        self.position.0 += (point.0
                            * (1.0 - self.image.size().0 as f64 / old_size.0 as f64))
                            .round() as i32;
                        self.position.1 += (point.1
                            * (1.0 - self.image.size().1 as f64 / old_size.1 as f64))
                            .round() as i32;
                        // 1. 【贴图】【滚轮锚点】已有移动从新大小继续，避免下一次指针事件恢复旧倍率
                        if let Some(drag) = &mut self.drag {
                            drag.start = crate::features::pinned_image::placement::Geometry::new(
                                self.position,
                                self.image.size(),
                            );
                            drag.pointer = drag.global(position);
                        }
                        self.resize(false, qh);
                    }
                }
                _ => {}
            }
        }
    }
}

impl KeyboardHandler for Pin {
    /// 【贴图】【按键按下】将 event 交给上下文快捷键处理
    fn press_key(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        self.key(event, qh);
    }
    /// 【贴图】【重复按键】贴图动作不重复执行，菜单导航等待下一次按下
    fn repeat_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        _: KeyEvent,
    ) {
    }
    /// 【贴图】【修饰键同步】保存 modifiers 供复制、保存和粘贴判断
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
        self.mods = crate::backend::wayland::keysym::mods(&modifiers);
    }
    /// 【贴图】【键盘进入】进入表面不修改选区或图像
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
    }
    /// 【贴图】【键盘离开】清除修饰键及未完成拖动，保持已经选择的文字
    fn leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: &wl_surface::WlSurface,
        _: u32,
    ) {
        self.mods = Mods::default();
        self.selecting = false;
    }
    /// 【贴图】【按键释放】释放键不重复触发窗口动作
    fn release_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        _: KeyEvent,
    ) {
    }
    /// 【贴图】【重复设置】当前贴图不使用定时按键重复
    fn update_repeat_info(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: RepeatInfo,
    ) {
    }
}

impl SeatHandler for Pin {
    /// 【贴图】【座席状态】返回当前输入座席状态
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seats
    }
    /// 【贴图】【输入绑定】为 seat 的 capability 创建设备，不重复绑定同类设备
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
                self.keyboard = self.seats.get_keyboard(qh, &seat, None).ok();
            }
            _ => {}
        }
    }
    /// 【贴图】【座席新增】等待能力通知后绑定设备
    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
    /// 【贴图】【能力移除】释放 seat 对应设备，并清理未完成拖动
    fn remove_capability(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        use wayland_client::Proxy;
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
                self.finish_gesture(qh);
            }
            Capability::Keyboard => {
                if let Some(keyboard) = self.keyboard.take()
                    && keyboard.version() >= 3
                {
                    keyboard.release();
                }
                self.mods = Mods::default();
            }
            _ => {}
        }
    }
    /// 【贴图】【座席移除】释放 seat 的指针与键盘
    fn remove_seat(&mut self, conn: &Connection, qh: &QueueHandle<Self>, seat: wl_seat::WlSeat) {
        self.remove_capability(conn, qh, seat.clone(), Capability::Pointer);
        self.remove_capability(conn, qh, seat, Capability::Keyboard);
    }
}
