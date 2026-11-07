//! 【贴图】【原生菜单】真正的父子 xdg-popup，可越出贴图范围，按层销毁并交付动作

use super::{
    menu_draw::{self, Row},
    render::buffer,
    runtime::{Pin, Shell},
};
use crate::{
    AppResult,
    features::pinned_image::menu::{self, PinAction},
};
use smithay_client_toolkit::{
    shell::xdg::{
        XdgPositioner, XdgSurface,
        popup::{Popup, PopupConfigure, PopupHandler},
    },
    shm::slot::Buffer,
};
use tiny_skia::Pixmap;
use wayland_client::{Connection, QueueHandle};
use wayland_protocols::{
    wp::viewporter::client::wp_viewport::WpViewport,
    xdg::shell::client::xdg_positioner::{Anchor, ConstraintAdjustment, Gravity},
};

pub(super) struct PinPopup {
    pub child: Option<Box<PinPopup>>,
    pub popup: Popup,
    pub hovered: Option<usize>,
    pub armed: Option<usize>,
    pub buffers: Vec<Buffer>,
    pub configured: bool,
    pub viewport: Option<WpViewport>,
    pub language: bool,
    pub grab_serial: u32,
}

impl PinPopup {
    /// 【贴图】【弹层尺寸】返回当前主菜单或语言子菜单的逻辑尺寸，无参数，不采用父贴图边界
    pub fn size(&self) -> (u32, u32) {
        (
            menu::WIDTH,
            if self.language {
                menu::PADDING * 2 + menu::ROW * 9
            } else {
                menu::height()
            },
        )
    }
    /// 【贴图】【弹层命中】按 position 返回当前菜单可命中行索引，分隔线和边框无动作
    pub fn hit(&self, position: (f64, f64)) -> Option<usize> {
        if !self.language {
            return menu::hit(position);
        }
        if position.0 < f64::from(menu::PADDING)
            || position.0 >= f64::from(menu::WIDTH - menu::PADDING)
            || position.1 < f64::from(menu::PADDING)
        {
            return None;
        }
        let row = ((position.1 - f64::from(menu::PADDING)) / f64::from(menu::ROW)).floor() as usize;
        (row < 9).then_some(row)
    }
}

impl Drop for PinPopup {
    /// 【贴图】【菜单表面释放】先销毁子菜单再销毁本层视口，工具包随后释放本层 xdg-popup，无参数
    fn drop(&mut self) {
        self.child = None;
        if let Some(viewport) = self.viewport.take() {
            viewport.destroy();
        }
    }
}

impl Pin {
    /// 【贴图】【菜单打开】用 position 和真实 serial 创建根抓取菜单，qh 绑定协议，先关闭旧菜单
    pub fn open_menu(
        &mut self,
        qh: &QueueHandle<Self>,
        position: (f64, f64),
        serial: u32,
    ) -> AppResult {
        self.popup = None;
        self.finish_gesture(qh);
        self.clicks.clear();
        let positioner = XdgPositioner::new(&self.xdg)?;
        positioner.set_size(menu::WIDTH as i32, menu::height() as i32);
        positioner.set_anchor_rect(position.0.round() as i32, position.1.round() as i32, 1, 1);
        positioner.set_anchor(Anchor::BottomRight);
        positioner.set_gravity(Gravity::BottomRight);
        positioner.set_constraint_adjustment(
            ConstraintAdjustment::SlideX
                | ConstraintAdjustment::SlideY
                | ConstraintAdjustment::FlipX
                | ConstraintAdjustment::FlipY,
        );
        let parent = match &self.shell {
            Shell::Window(window) => Some(window.xdg_surface()),
            Shell::Layer(_) => None,
        };
        let popup = Popup::from_surface(
            parent,
            &positioner,
            qh,
            self.compositor.create_surface(qh),
            &self.xdg,
        )?;
        if let Shell::Layer(layer) = &self.shell {
            layer.get_popup(popup.xdg_popup());
        }
        if let Some(seat) = &self.pointer_seat {
            popup.xdg_popup().grab(seat, serial);
        }
        let viewport = self.viewporter.as_ref().map(|manager| {
            let viewport = manager.get_viewport(popup.wl_surface(), qh, ());
            viewport.set_destination(menu::WIDTH as i32, menu::height() as i32);
            viewport
        });
        popup.wl_surface().commit();
        self.popup = Some(PinPopup {
            child: None,
            popup,
            hovered: None,
            armed: None,
            buffers: Vec::new(),
            configured: false,
            viewport,
            language: false,
            grab_serial: serial,
        });
        Ok(())
    }

    /// 【贴图】【菜单动作可用】按 action 检查协议、文字选择与翻译任务，不启动模型或访问服务
    pub fn menu_enabled(&self, action: PinAction) -> bool {
        menu::enabled(
            action,
            self.layers.is_some(),
            self.text.has_selection() && self.text_selection,
            self.text.is_active(),
            self.translation_task.is_some(),
            !self.translation.blocks.is_empty(),
        )
    }

    /// 【贴图】【菜单内容】按 language 返回当前主菜单或语言列表状态，勾选和切换标签对应实际显示
    fn menu_rows(&self, language: bool) -> Vec<Row> {
        let ui = crate::ui::language::Language::current();
        if language {
            return crate::translation::language::OPTIONS
                .iter()
                .enumerate()
                .map(|(index, target)| Row {
                    label: crate::translation::language::labels(ui)[index].into(),
                    enabled: true,
                    checked: *target == self.translation.target,
                    danger: false,
                    submenu: false,
                })
                .collect();
        }
        menu::ITEMS
            .iter()
            .map(|item| {
                let action = item.action;
                let label =
                    if action == Some(PinAction::ToggleTranslation) && self.translation.active {
                        "Show Original Text"
                    } else {
                        item.label
                    };
                Row {
                    label: ui.text(label).into(),
                    enabled: action.is_some_and(|action| self.menu_enabled(action)),
                    checked: action == Some(PinAction::AlwaysOnTop) && self.image.always_on_top
                        || action == Some(PinAction::TextSelection) && self.text_selection
                        || action == Some(PinAction::ToggleTranslation) && self.translation.active,
                    danger: action == Some(PinAction::Close),
                    submenu: action == Some(PinAction::TargetLanguage),
                }
            })
            .collect()
    }

    /// 【贴图】【单层提交】按 popup 绘制并提交独立小缓冲，父子菜单不共享可写像素
    fn draw_popup(&mut self, popup: &mut PinPopup) -> AppResult {
        if !popup.configured {
            return Ok(());
        }
        let scale = if popup.viewport.is_some() {
            self.render_scale
        } else {
            self.render_scale.max(1.0).ceil()
        };
        let size = popup.size();
        let pixels = (
            (size.0 as f32 * scale).ceil() as u32,
            (size.1 as f32 * scale).ceil() as u32,
        );
        let Some(index) = buffer(&mut self.pool, &mut popup.buffers, pixels)? else {
            return Ok(());
        };
        let rows = self.menu_rows(popup.language);
        let mut canvas = Pixmap::new(pixels.0, pixels.1).ok_or("cannot allocate pin menu")?;
        menu_draw::draw(
            &mut canvas,
            &rows,
            popup.hovered,
            size,
            scale,
            &mut self.font_system,
            &mut self.swash,
            &self.icons,
        );
        let buffer = &popup.buffers[index];
        crate::utils::copy_swizzled(
            buffer
                .canvas(&mut self.pool)
                .ok_or("pin menu buffer is active")?,
            canvas.data(),
        );
        if popup.viewport.is_none() {
            popup.popup.wl_surface().set_buffer_scale(scale as i32);
        }
        buffer.attach_to(popup.popup.wl_surface())?;
        popup
            .popup
            .wl_surface()
            .damage_buffer(0, 0, pixels.0 as i32, pixels.1 as i32);
        popup.popup.wl_surface().commit();
        Ok(())
    }

    /// 【贴图】【菜单绘制】提交主菜单和实际子菜单的独立缓冲，无参数，不执行翻译或配置修改
    pub fn draw_menu(&mut self) -> AppResult {
        let Some(mut popup) = self.popup.take() else {
            return Ok(());
        };
        let result = (|| {
            self.draw_popup(&mut popup)?;
            if let Some(child) = &mut popup.child {
                self.draw_popup(child)?;
            }
            Ok(())
        })();
        self.popup = Some(popup);
        result
    }
}

impl PopupHandler for Pin {
    /// 【贴图】【菜单配置】配置当前父菜单或子菜单，旧表面事件不改变当前链，绘制失败释放本链
    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        popup: &Popup,
        _: PopupConfigure,
    ) {
        if let Some(current) = &mut self.popup {
            if &current.popup == popup {
                current.configured = true;
            } else if let Some(child) = &mut current.child
                && &child.popup == popup
            {
                child.configured = true;
            } else {
                return;
            }
        } else {
            return;
        }
        if let Err(error) = self.draw_menu() {
            log::error!("【贴图】【右键菜单】菜单绘制失败: {error}");
            self.popup = None;
        }
    }
    /// 【贴图】【菜单取消】合成器只取消子层时保留父菜单，取消父菜单时按序释放全部抓取
    fn done(&mut self, _: &Connection, _: &QueueHandle<Self>, popup: &Popup) {
        if self
            .popup
            .as_ref()
            .is_some_and(|current| &current.popup == popup)
        {
            self.popup = None;
        } else if let Some(current) = &mut self.popup
            && current
                .child
                .as_ref()
                .is_some_and(|child| &child.popup == popup)
        {
            current.child = None;
        }
    }
}
