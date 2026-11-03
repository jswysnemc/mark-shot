//! 【贴图】【原生菜单】真正的 xdg-popup 可伸出小贴图边界，合成器负责抓取与屏幕避让

use super::{
    render::buffer,
    runtime::{Pin, Shell},
};
use crate::{
    AppResult,
    features::pinned_image::menu::{self, PinAction},
    renderer::{Painter, text::TextStyle},
    theme::Rgba,
};
use smithay_client_toolkit::{
    shell::xdg::{
        XdgPositioner, XdgSurface,
        popup::{Popup, PopupConfigure, PopupHandler},
    },
    shm::slot::Buffer,
};
use tiny_skia::{Pixmap, Rect, Stroke};
use wayland_client::{Connection, QueueHandle};
use wayland_protocols::wp::viewporter::client::wp_viewport::WpViewport;
use wayland_protocols::xdg::shell::client::xdg_positioner::{
    Anchor, ConstraintAdjustment, Gravity,
};

pub(super) struct PinPopup {
    pub popup: Popup,
    pub hovered: Option<usize>,
    pub armed: Option<usize>,
    pub buffers: Vec<Buffer>,
    pub configured: bool,
    pub viewport: Option<WpViewport>,
}

impl Drop for PinPopup {
    /// 【贴图】【菜单表面释放】先销毁视口再由工具包销毁菜单表面；不保留已经关闭的菜单资源
    fn drop(&mut self) {
        if let Some(viewport) = self.viewport.take() {
            viewport.destroy();
        }
    }
}

impl Pin {
    /// 【贴图】【菜单打开】用 position 和真实 serial 创建抓取菜单；qh 绑定协议，先关闭旧菜单
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
            popup,
            hovered: None,
            armed: None,
            buffers: Vec::new(),
            configured: false,
            viewport,
        });
        Ok(())
    }

    /// 【贴图】【菜单动作可用】按 action 检查协议及文字选择状态；不启动模型任务
    pub fn menu_enabled(&self, action: PinAction) -> bool {
        menu::enabled(
            action,
            self.layers.is_some(),
            self.text.has_selection() && self.text_selection,
            self.text.is_active(),
        )
    }

    /// 【贴图】【菜单绘制】重绘当前 popup 的小型缓冲；不修改贴图图像或启动 OCR
    pub fn draw_menu(&mut self) -> AppResult {
        let Some(mut popup) = self.popup.take() else {
            return Ok(());
        };
        let result = (|| {
            if !popup.configured {
                return Ok(());
            }
            let scale = if popup.viewport.is_some() {
                self.render_scale
            } else {
                self.render_scale.max(1.0).ceil()
            };
            let size = (
                (menu::WIDTH as f32 * scale).ceil() as u32,
                (menu::height() as f32 * scale).ceil() as u32,
            );
            let Some(index) = buffer(&mut self.pool, &mut popup.buffers, size)? else {
                return Ok(());
            };
            let mut canvas = Pixmap::new(size.0, size.1).ok_or("cannot allocate pin menu")?;
            let mut painter = Painter::new(&mut canvas)
                .with_scale(scale)
                .with_icons(&self.icons)
                .with_text(&mut self.font_system, &mut self.swash);
            painter.fill_rounded(
                Rect::from_xywh(
                    0.5,
                    0.5,
                    menu::WIDTH as f32 - 1.0,
                    menu::height() as f32 - 1.0,
                )
                .unwrap(),
                8.0,
                Rgba(24, 27, 35, 246),
            );
            if let Some(path) = crate::renderer::paths::rounded_rect_path(
                &Rect::from_xywh(
                    0.5,
                    0.5,
                    menu::WIDTH as f32 - 1.0,
                    menu::height() as f32 - 1.0,
                )
                .unwrap(),
                8.0,
                true,
                true,
                true,
                true,
            ) {
                painter.stroke_path(
                    &path,
                    Rgba(255, 255, 255, 30),
                    &Stroke {
                        width: 1.0,
                        ..Stroke::default()
                    },
                );
            }
            let mut top = menu::PADDING as f32;
            for (row, item) in menu::ITEMS.iter().enumerate() {
                let Some(action) = item.action else {
                    painter.fill_rounded(
                        Rect::from_xywh(8.0, top + 4.5, menu::WIDTH as f32 - 16.0, 1.0).unwrap(),
                        0.0,
                        Rgba(255, 255, 255, 22),
                    );
                    top += menu::SEPARATOR as f32;
                    continue;
                };
                let enabled = menu::enabled(
                    action,
                    self.layers.is_some(),
                    self.text.has_selection() && self.text_selection,
                    self.text.is_active(),
                );
                let danger = action == PinAction::Close;
                let hovered = popup.hovered == Some(row) && enabled;
                let rect = Rect::from_xywh(
                    menu::PADDING as f32,
                    top,
                    (menu::WIDTH - menu::PADDING * 2) as f32,
                    menu::ROW as f32,
                )
                .unwrap();
                if hovered {
                    painter.fill_rounded(
                        rect,
                        5.0,
                        if danger {
                            Rgba(239, 68, 68, 52)
                        } else {
                            Rgba(45, 212, 191, 46)
                        },
                    );
                }
                let checked = action == PinAction::AlwaysOnTop && self.image.always_on_top
                    || action == PinAction::TextSelection && self.text_selection;
                if checked {
                    painter.icon(
                        crate::ui::icons::CHECK,
                        14.0,
                        12.0,
                        top + 7.0,
                        Rgba(94, 234, 212, 255),
                    );
                }
                let ink = if !enabled {
                    Rgba(100, 116, 139, 255)
                } else if danger && hovered {
                    Rgba(252, 165, 165, 255)
                } else if danger {
                    Rgba(248, 113, 113, 255)
                } else if hovered {
                    Rgba(248, 250, 252, 255)
                } else {
                    Rgba(229, 231, 235, 255)
                };
                painter.text(
                    Rect::from_xywh(34.0, top, menu::WIDTH as f32 - 44.0, menu::ROW as f32)
                        .unwrap(),
                    item.label,
                    TextStyle::new(12.0, ink),
                );
                top += menu::ROW as f32;
            }
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
                .damage_buffer(0, 0, size.0 as i32, size.1 as i32);
            popup.popup.wl_surface().commit();
            Ok(())
        })();
        self.popup = Some(popup);
        result
    }
}

impl PopupHandler for Pin {
    /// 【贴图】【菜单配置】收到 popup 首次配置后绘制菜单；错误关闭菜单，不结束贴图窗口
    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        popup: &Popup,
        _: PopupConfigure,
    ) {
        if let Some(current) = &mut self.popup {
            if &current.popup != popup {
                return;
            }
            current.configured = true;
        }
        if let Err(error) = self.draw_menu() {
            log::error!("【贴图】【右键菜单】菜单绘制失败: {error}");
            self.popup = None;
        }
    }
    /// 【贴图】【菜单取消】合成器取消 popup 抓取后释放菜单，保留贴图与文字选择
    fn done(&mut self, _: &Connection, _: &QueueHandle<Self>, popup: &Popup) {
        if self
            .popup
            .as_ref()
            .is_some_and(|current| &current.popup == popup)
        {
            self.popup = None;
        }
    }
}
