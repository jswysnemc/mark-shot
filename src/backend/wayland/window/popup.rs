//! 【原生窗口】【弹层生命周期】独立 xdg-popup 共享有界分配器，销毁抓取后才交付编辑动作

use super::{WindowPopupAction, runtime::NativeWindow, surface::SurfaceId};
use crate::{
    AppResult,
    backend::wayland::utils::buffers,
    types::popup::{PopupEvent, PopupFrame, validate},
};
use smithay_client_toolkit::{
    compositor::FrameCallbackData,
    shell::xdg::{
        XdgPositioner, XdgSurface,
        popup::{Popup, PopupConfigure, PopupHandler},
    },
    shm::slot::Buffer,
};
use wayland_client::{
    Connection, QueueHandle,
    protocol::{wl_seat::WlSeat, wl_surface::WlSurface},
};
use wayland_protocols::{
    wp::viewporter::client::wp_viewport::WpViewport,
    xdg::shell::client::xdg_positioner::{Anchor, ConstraintAdjustment, Gravity},
};

pub(super) struct WindowPopup {
    pub id: u64,
    pub popup: Popup,
    pub keyboard_focused: bool,
    configured: bool,
    size: (u32, u32),
    scale: f32,
    pixel_size: (u32, u32),
    pixels: Vec<u8>,
    buffers: Vec<Buffer>,
    viewport: Option<WpViewport>,
    dirty: bool,
    frame_pending: bool,
}

impl Drop for WindowPopup {
    /// 【原生窗口】【弹层回收】先销毁视区，再由工具包销毁角色和表面，无参数、无返回值
    fn drop(&mut self) {
        if let Some(viewport) = self.viewport.take() {
            viewport.destroy();
        }
    }
}

impl NativeWindow {
    /// 【原生窗口】【弹层同步】用 qh 同步父内容的菜单；grab 必须来自当前真实按下及座席，不保存可复用旧序列号，无返回值
    pub fn sync_popup(&mut self, qh: &QueueHandle<Self>, grab: Option<(&WlSeat, u32)>) {
        let requested = self.content.popup_id();
        if let Some(id) = self.popup.as_ref().map(|popup| popup.id)
            && (requested != Some(id) || self.popup.as_ref().unwrap().scale != self.main.scale)
        {
            self.finish_popup(id, None);
        }
        let Some(id) = self.content.popup_id() else {
            return;
        };
        if self.dialog.is_some() || self.exit {
            self.content.popup_closed(id, None);
            return;
        }
        let Some(frame) = self.content.popup_frame(self.main.scale) else {
            return;
        };
        let result = update(
            &mut self.popup,
            frame,
            grab,
            &self.main.window,
            &self.compositor,
            &self.shell,
            self.viewporter.as_ref(),
            qh,
        );
        if let Err(error) = result {
            log::error!("【原生窗口】【弹层同步】{error}");
            self.popup.take();
            let action = self.content.popup_closed(id, None);
            self.apply(SurfaceId::Main, action, None);
        }
        self.reconcile_text_input();
    }

    /// 【原生窗口】【弹层完成】仅关闭存活 id，先销毁原生抓取，再交付 action；空值表示取消，无返回值
    pub fn finish_popup(&mut self, id: u64, action: Option<usize>) {
        if !self.popup.as_ref().is_some_and(|popup| popup.id == id) {
            return;
        }
        // 1. 【原生窗口】【动作顺序】先排入销毁请求，复制和正文操作不能发生在仍持有原生抓取的菜单中
        self.popup.take();
        log::info!("【原生窗口】【弹层关闭】id={id}, action={action:?}");
        let action = self.content.popup_closed(id, action);
        self.apply(SurfaceId::Main, action, None);
        self.reconcile_text_input();
    }

    /// 【原生窗口】【弹层分派】只向当前 id 交付 event，动作请求先回收原生表面；过期输入不影响新菜单，无返回值
    pub fn popup_dispatch(&mut self, id: u64, event: PopupEvent) {
        if !self.popup.as_ref().is_some_and(|popup| popup.id == id) {
            return;
        }
        match self.content.popup_event(id, event) {
            WindowPopupAction::None => {}
            WindowPopupAction::Redraw => {}
            WindowPopupAction::Close => self.finish_popup(id, None),
            WindowPopupAction::Activate(index) => self.finish_popup(id, Some(index)),
        }
    }

    /// 【原生窗口】【弹层绘制】用 qh 提交最新像素，配置和帧许可不足时保留脏帧；返回缓冲错误，无空闲轮询
    pub fn paint_popup(&mut self, qh: &QueueHandle<Self>) -> AppResult {
        let Some(popup) = self.popup.as_mut() else {
            return Ok(());
        };
        if !popup.configured || !popup.dirty || popup.frame_pending {
            return Ok(());
        }
        let Some(index) = buffers::available(&mut self.pool, &mut popup.buffers, popup.pixel_size)?
        else {
            return Ok(());
        };
        let buffer = &popup.buffers[index];
        let canvas = buffer
            .canvas(&mut self.pool)
            .ok_or("popup buffer remains active")?;
        crate::utils::copy_swizzled(canvas, &popup.pixels);
        if popup.viewport.is_none() {
            popup
                .popup
                .wl_surface()
                .set_buffer_scale(popup.scale as i32);
        }
        buffer.attach_to(popup.popup.wl_surface())?;
        popup.popup.wl_surface().damage_buffer(
            0,
            0,
            popup.pixel_size.0 as i32,
            popup.pixel_size.1 as i32,
        );
        popup
            .popup
            .wl_surface()
            .frame(qh, FrameCallbackData(popup.popup.wl_surface().clone()));
        popup.popup.wl_surface().commit();
        popup.dirty = false;
        popup.frame_pending = true;
        Ok(())
    }

    /// 【原生窗口】【弹层帧许可】surface 匹配当前菜单时解除提交限制，返回是否消费；过期表面不影响新菜单
    pub fn popup_frame_done(&mut self, surface: &WlSurface) -> bool {
        let Some(popup) = self
            .popup
            .as_mut()
            .filter(|popup| popup.popup.wl_surface() == surface)
        else {
            return false;
        };
        popup.frame_pending = false;
        true
    }
}

/// 【原生窗口】【弹层创建】以 frame、grab 和 parent 建立或更新菜单，compositor、shell、viewporter、qh 提供协议资源；返回校验或创建错误
fn update(
    current: &mut Option<WindowPopup>,
    frame: PopupFrame<'_>,
    grab: Option<(&WlSeat, u32)>,
    parent: &smithay_client_toolkit::shell::xdg::window::Window,
    compositor: &smithay_client_toolkit::compositor::CompositorState,
    shell: &smithay_client_toolkit::shell::xdg::XdgShell,
    viewporter: Option<&wayland_protocols::wp::viewporter::client::wp_viewporter::WpViewporter>,
    qh: &QueueHandle<NativeWindow>,
) -> AppResult {
    validate(&frame)?;
    if viewporter.is_none() && (frame.scale < 1.0 || frame.scale.fract() != 0.0) {
        return Err("fractional popup scale requires viewporter".into());
    }
    if current.is_none() {
        let (seat, serial) = grab.ok_or("popup request has no current input grab")?;
        let positioner = XdgPositioner::new(shell)?;
        positioner.set_size(frame.size.0 as i32, frame.size.1 as i32);
        positioner.set_anchor_rect(
            frame.anchor.0.round() as i32,
            frame.anchor.1.round() as i32,
            1,
            1,
        );
        positioner.set_anchor(Anchor::TopLeft);
        positioner.set_gravity(Gravity::BottomRight);
        positioner.set_constraint_adjustment(
            ConstraintAdjustment::SlideX
                | ConstraintAdjustment::SlideY
                | ConstraintAdjustment::FlipX
                | ConstraintAdjustment::FlipY,
        );
        let popup = Popup::from_surface(
            Some(parent.xdg_surface()),
            &positioner,
            qh,
            compositor.create_surface(qh),
            shell,
        )?;
        popup.xdg_popup().grab(seat, serial);
        let viewport = viewporter.map(|manager| {
            let viewport = manager.get_viewport(popup.wl_surface(), qh, SurfaceId::Main);
            viewport.set_destination(frame.size.0 as i32, frame.size.1 as i32);
            viewport
        });
        popup.wl_surface().commit();
        log::info!("【原生窗口】【弹层打开】id={}, serial={serial}", frame.id);
        *current = Some(WindowPopup {
            id: frame.id,
            popup,
            keyboard_focused: false,
            configured: false,
            size: frame.size,
            scale: frame.scale,
            pixel_size: frame.pixel_size,
            pixels: Vec::new(),
            buffers: Vec::new(),
            viewport,
            dirty: true,
            frame_pending: false,
        });
    }
    let popup = current.as_mut().unwrap();
    if popup.id != frame.id
        || popup.size != frame.size
        || popup.scale != frame.scale
        || popup.pixel_size != frame.pixel_size
    {
        return Err("popup identity or dimensions changed during grab".into());
    }
    popup.pixels.clear();
    popup.pixels.extend_from_slice(frame.pixels);
    popup.dirty = true;
    Ok(())
}

impl PopupHandler for NativeWindow {
    /// 【原生窗口】【弹层配置】接受 popup 的 configure，用 qh 提交独立首帧；位置交给内容用于命中，无返回值
    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        popup: &Popup,
        configure: PopupConfigure,
    ) {
        let Some(current) = self
            .popup
            .as_mut()
            .filter(|current| &current.popup == popup)
        else {
            return;
        };
        current.configured = true;
        let id = current.id;
        self.popup_dispatch(
            id,
            PopupEvent::Configured {
                position: configure.position,
            },
        );
    }

    /// 【原生窗口】【合成器取消】仅关闭 popup 对应的当前菜单，旧 done 消息不能关闭新会话，无返回值
    fn done(&mut self, _: &Connection, _: &QueueHandle<Self>, popup: &Popup) {
        if let Some(id) = self
            .popup
            .as_ref()
            .filter(|current| &current.popup == popup)
            .map(|current| current.id)
        {
            self.finish_popup(id, None);
        }
    }
}
