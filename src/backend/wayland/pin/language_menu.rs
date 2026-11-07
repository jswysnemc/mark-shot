//! 【贴图翻译】【语言子菜单】真实父子表面，右键不执行选择，键盘左右导航保持父链

use super::{popup::PinPopup, runtime::Pin};
use crate::{
    AppResult,
    features::pinned_image::menu::{self, PinAction},
    keys::Key,
};
use smithay_client_toolkit::{
    seat::pointer::{PointerEvent, PointerEventKind},
    shell::xdg::{XdgPositioner, popup::Popup},
};
use wayland_client::{QueueHandle, protocol::wl_surface};
use wayland_protocols::{
    wp::cursor_shape::v1::client::wp_cursor_shape_device_v1::Shape,
    xdg::shell::client::xdg_positioner::{Anchor, ConstraintAdjustment, Gravity},
};

const LEFT: u32 = 0x110;

impl Pin {
    /// 【贴图翻译】【子菜单创建】在当前语言父行旁创建实际抓取子菜单，qh 绑定协议，父菜单保持显示
    pub fn open_language_menu(&mut self, qh: &QueueHandle<Self>) -> AppResult {
        let Some(root) = &self.popup else {
            return Ok(());
        };
        if root.child.is_some() {
            return Ok(());
        }
        let row = menu::ITEMS
            .iter()
            .position(|item| item.action == Some(PinAction::TargetLanguage))
            .unwrap();
        let size = (menu::WIDTH, menu::PADDING * 2 + menu::ROW * 9);
        let positioner = XdgPositioner::new(&self.xdg)?;
        positioner.set_size(size.0 as i32, size.1 as i32);
        positioner.set_anchor_rect(
            menu::PADDING as i32,
            menu::row_top(row).unwrap() as i32,
            (menu::WIDTH - menu::PADDING * 2) as i32,
            menu::ROW as i32,
        );
        positioner.set_anchor(Anchor::TopRight);
        positioner.set_gravity(Gravity::BottomRight);
        positioner.set_constraint_adjustment(
            ConstraintAdjustment::SlideX
                | ConstraintAdjustment::SlideY
                | ConstraintAdjustment::FlipX
                | ConstraintAdjustment::FlipY,
        );
        let popup = Popup::from_surface(
            Some(root.popup.xdg_surface()),
            &positioner,
            qh,
            self.compositor.create_surface(qh),
            &self.xdg,
        )?;
        if let Some(seat) = &self.pointer_seat {
            popup.xdg_popup().grab(seat, root.grab_serial);
        }
        let viewport = self.viewporter.as_ref().map(|manager| {
            let viewport = manager.get_viewport(popup.wl_surface(), qh, ());
            viewport.set_destination(size.0 as i32, size.1 as i32);
            viewport
        });
        let grab_serial = root.grab_serial;
        popup.wl_surface().commit();
        self.popup.as_mut().unwrap().child = Some(Box::new(PinPopup {
            child: None,
            popup,
            hovered: None,
            armed: None,
            buffers: Vec::new(),
            configured: false,
            viewport,
            language: true,
            grab_serial,
        }));
        Ok(())
    }

    /// 【贴图翻译】【菜单表面判断】按 surface 返回是否为子菜单表面，主菜单返回 false，其他表面返回空值
    fn menu_surface(&self, surface: &wl_surface::WlSurface) -> Option<bool> {
        let root = self.popup.as_ref()?;
        if root
            .child
            .as_ref()
            .is_some_and(|child| child.popup.wl_surface() == surface)
        {
            Some(true)
        } else if root.popup.wl_surface() == surface {
            Some(false)
        } else {
            None
        }
    }

    /// 【贴图翻译】【菜单行交付】按 child 和 index 执行当前层动作，语言选择先销毁整条菜单链再保存配置
    fn menu_activate(&mut self, child: bool, index: usize, qh: &QueueHandle<Self>) {
        if child {
            self.popup = None;
            let result = (|| {
                self.connection.flush()?;
                self.choose_language(index)
            })();
            self.report(result);
        } else if let Some(action) = menu::ITEMS.get(index).and_then(|item| item.action) {
            if action == PinAction::TargetLanguage {
                let result = self.open_language_menu(qh);
                self.report(result);
            } else {
                self.action(action, qh);
            }
        }
    }

    /// 【贴图翻译】【菜单指针】处理 event 对应父子表面并返回是否消费，左键完整按下释放才执行，主图像不接收菜单指针
    pub fn menu_pointer(&mut self, event: &PointerEvent, qh: &QueueHandle<Self>) -> bool {
        let Some(child) = self.menu_surface(&event.surface) else {
            return false;
        };
        let position = event.position;
        match event.kind {
            PointerEventKind::Enter { serial } | PointerEventKind::Motion { time: serial } => {
                if matches!(event.kind, PointerEventKind::Enter { .. }) {
                    self.serial = serial;
                    if let Some(cursor) = &self.cursor {
                        cursor.set_shape(serial, Shape::Default);
                    }
                }
                let popup = self.popup.as_mut().unwrap();
                let current = if child {
                    popup.child.as_deref_mut().unwrap()
                } else {
                    popup
                };
                current.hovered = current.hit(position);
                if !child {
                    let target = self
                        .popup
                        .as_ref()
                        .and_then(|popup| popup.hovered)
                        .and_then(|index| menu::ITEMS[index].action)
                        == Some(PinAction::TargetLanguage);
                    if target {
                        let result = self.open_language_menu(qh);
                        self.report(result);
                    } else if self
                        .popup
                        .as_ref()
                        .is_some_and(|popup| popup.hovered.is_some())
                    {
                        self.popup.as_mut().unwrap().child = None;
                    }
                }
                let result = self.draw_menu();
                self.report(result);
            }
            PointerEventKind::Press { button: LEFT, .. } => {
                let popup = self.popup.as_mut().unwrap();
                let current = if child {
                    popup.child.as_deref_mut().unwrap()
                } else {
                    popup
                };
                current.armed = current.hit(position);
            }
            PointerEventKind::Release { button: LEFT, .. } => {
                let popup = self.popup.as_mut().unwrap();
                let current = if child {
                    popup.child.as_deref_mut().unwrap()
                } else {
                    popup
                };
                let hit = current.hit(position);
                let armed = current.armed.take();
                if let Some(index) = hit.filter(|index| Some(*index) == armed) {
                    self.menu_activate(child, index, qh);
                }
            }
            PointerEventKind::Press { .. } => self.popup = None,
            PointerEventKind::Leave { .. } if child => {
                if let Some(child) = self
                    .popup
                    .as_mut()
                    .and_then(|popup| popup.child.as_deref_mut())
                {
                    child.hovered = None;
                }
            }
            _ => {}
        }
        true
    }

    /// 【贴图翻译】【菜单键盘】按 key 导航当前父子层，左右键开关子菜单，Escape 先关闭子层，返回是否消费
    pub fn menu_key(&mut self, key: Option<Key>, qh: &QueueHandle<Self>) -> bool {
        let Some(popup) = &self.popup else {
            return false;
        };
        let child = popup.child.is_some();
        let active = if child {
            popup.child.as_ref().unwrap().hovered
        } else {
            popup.hovered
        };
        match key {
            Some(Key::Escape | Key::Left) if child => self.popup.as_mut().unwrap().child = None,
            Some(Key::Escape) => self.popup = None,
            Some(Key::Right) if !child => {
                if active.and_then(|index| menu::ITEMS[index].action)
                    == Some(PinAction::TargetLanguage)
                {
                    let result = self.open_language_menu(qh);
                    self.report(result);
                }
            }
            Some(Key::Enter | Key::Space) => {
                if let Some(index) = active {
                    self.menu_activate(child, index, qh);
                }
            }
            Some(Key::Home | Key::End) => {
                let length = if child { 9 } else { menu::ITEMS.len() };
                let indices = (0..length).map(|index| {
                    if key == Some(Key::Home) {
                        index
                    } else {
                        length - 1 - index
                    }
                });
                let next = indices.into_iter().find(|index| {
                    child
                        || menu::ITEMS[*index]
                            .action
                            .is_some_and(|action| self.menu_enabled(action))
                });
                let popup = self.popup.as_mut().unwrap();
                let current = if child {
                    popup.child.as_deref_mut().unwrap()
                } else {
                    popup
                };
                current.hovered = next;
            }
            Some(Key::Up | Key::Down) => {
                let forward = key == Some(Key::Down);
                let length = if child { 9 } else { menu::ITEMS.len() };
                let start = active.unwrap_or(if forward { length - 1 } else { 0 });
                let next = (1..=length)
                    .map(|step| {
                        if forward {
                            (start + step) % length
                        } else {
                            (start + length - step) % length
                        }
                    })
                    .find(|index| {
                        child
                            || menu::ITEMS[*index]
                                .action
                                .is_some_and(|action| self.menu_enabled(action))
                    });
                let popup = self.popup.as_mut().unwrap();
                let current = if child {
                    popup.child.as_deref_mut().unwrap()
                } else {
                    popup
                };
                current.hovered = next;
            }
            _ => {}
        }
        let result = self.draw_menu();
        self.report(result);
        true
    }
}
