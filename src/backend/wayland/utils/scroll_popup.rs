//! 【滚动截图】【标准弹出角色】以透明父窗口持有控制表面，使用合成器边界约束且不申请菜单抓取

use super::shm::{ShmBuffer, create_shm_buffer};
use crate::{AppResult, backend::wayland::overlay::state::OverlayState};
use smithay_client_toolkit::{
    compositor::CompositorState,
    shell::{
        WaylandSurface,
        xdg::{
            XdgPositioner, XdgShell, XdgSurface,
            popup::{ConfigureKind, Popup},
            window::{Window, WindowDecorations},
        },
    },
    shm::slot::SlotPool,
};
use wayland_client::{Proxy, QueueHandle, protocol::wl_surface::WlSurface};
use wayland_protocols::wp::viewporter::client::wp_viewport::WpViewport;
use wayland_protocols::xdg::shell::client::xdg_positioner::{
    Anchor, ConstraintAdjustment, Gravity,
};

pub struct ScrollPopup {
    pub popup: Popup,
    pub parent: Window,
    parent_viewport: Option<WpViewport>,
    parent_buffer: ShmBuffer,
    positioner: XdgPositioner,
    pub requested: (u32, u32),
    pub pending: bool,
    pub limits: Option<(u32, u32)>,
    token: u32,
    pub parent_size: (u32, u32),
}

impl ScrollPopup {
    /// 【滚动截图】【角色创建】按 size、surface 和工具包资源建立透明父窗口与控制角色，返回协议或缓冲错误
    pub fn new(
        size: (u32, u32),
        surface: WlSurface,
        parent: Window,
        parent_buffer: ShmBuffer,
        parent_size: (u32, u32),
        parent_viewport: Option<WpViewport>,
        shell: &XdgShell,
        qh: &QueueHandle<OverlayState>,
    ) -> AppResult<Self> {
        let positioner = XdgPositioner::new(shell)?;
        if positioner.version() < 3 {
            return Err("scroll control resizing requires xdg-shell version 3".into());
        }
        positioner.set_size(size.0 as i32, size.1 as i32);
        positioner.set_anchor_rect(0, 0, 1, 1);
        positioner.set_anchor(Anchor::TopLeft);
        positioner.set_gravity(Gravity::BottomRight);
        positioner.set_constraint_adjustment(
            ConstraintAdjustment::SlideX
                | ConstraintAdjustment::SlideY
                | ConstraintAdjustment::ResizeX
                | ConstraintAdjustment::ResizeY,
        );
        positioner.set_reactive();
        positioner.set_parent_size(parent_size.0 as i32, parent_size.1 as i32);
        let popup =
            Popup::from_surface(Some(parent.xdg_surface()), &positioner, qh, surface, shell)?;
        Ok(Self {
            popup,
            parent,
            parent_viewport,
            parent_buffer,
            positioner,
            requested: size,
            pending: true,
            limits: None,
            token: 0,
            parent_size,
        })
    }

    /// 【滚动截图】【父窗口创建】用 compositor、shell 和 qh 创建最小普通父窗口，返回待配置窗口，不提前创建子角色
    pub fn create_parent(
        compositor: &CompositorState,
        shell: &XdgShell,
        qh: &QueueHandle<OverlayState>,
    ) -> Window {
        let parent =
            shell.create_window(compositor.create_surface(qh), WindowDecorations::None, qh);
        parent.set_title("mark-shot");
        parent.set_app_id("mark-shot-scroll");
        parent.set_min_size(Some((1, 1)));
        parent.set_max_size(Some((1, 1)));
        parent.set_window_geometry(0, 0, 1, 1);
        parent
    }

    /// 【滚动截图】【父表面配置】按 size 和 pool 更新透明父缓冲，返回分配错误，不改变控制图像
    pub fn map_parent(&mut self, size: (u32, u32), pool: &mut SlotPool) -> AppResult {
        let pixels = if self.parent_viewport.is_some() {
            (1, 1)
        } else {
            size
        };
        let mut buffer = create_shm_buffer(pool, pixels.0, pixels.1)?;
        buffer.clear(pool);
        if let Some(viewport) = &self.parent_viewport {
            viewport.set_destination(size.0 as i32, size.1 as i32);
        }
        self.parent_buffer = buffer;
        self.parent_size = size;
        self.parent.set_window_geometry(0, 0, size.0, size.1);
        self.parent_buffer
            .buffer
            .attach_to(self.parent.wl_surface())?;
        self.parent
            .wl_surface()
            .damage_buffer(0, 0, pixels.0 as i32, pixels.1 as i32);
        self.parent.commit();
        self.positioner
            .set_parent_size(size.0 as i32, size.1 as i32);
        Ok(())
    }

    /// 【滚动截图】【尺寸请求】以 size 更新边界约束并等待配置，返回是否发起新请求，不提前提交不匹配的缓冲
    pub fn resize(&mut self, size: (u32, u32)) -> bool {
        if self.requested == size {
            return false;
        }
        self.requested = size;
        self.positioner.set_size(size.0 as i32, size.1 as i32);
        self.token = self.token.wrapping_add(1);
        self.pending = true;
        self.popup.reposition(&self.positioner, self.token);
        true
    }

    /// 【滚动截图】【控件重建】在 surface 上重建合成器因页面点击关闭的控件，shell 与 qh 使用同一连接，返回协议结果，父窗口和授权保持不变
    pub fn reopen(
        &mut self,
        surface: WlSurface,
        shell: &XdgShell,
        qh: &QueueHandle<OverlayState>,
    ) -> AppResult {
        self.popup = Popup::from_surface(
            Some(self.parent.xdg_surface()),
            &self.positioner,
            qh,
            surface,
            shell,
        )?;
        self.pending = true;
        self.token = 0;
        self.popup.wl_surface().commit();
        Ok(())
    }

    /// 【滚动截图】【配置完成】按 size 和 kind 确认当前请求，返回是否接受；过期响应不能解除新尺寸等待
    pub fn configured(&mut self, size: (u32, u32), kind: &ConfigureKind) -> bool {
        match kind {
            ConfigureKind::Initial if self.token != 0 => return false,
            ConfigureKind::Reposition { token } if *token != self.token => return false,
            ConfigureKind::Reactive if self.pending => return false,
            _ => {}
        }
        self.pending = false;
        if size.0 < self.requested.0 || size.1 < self.requested.1 {
            self.limits = Some(size);
        }
        self.popup
            .xdg_surface()
            .set_window_geometry(0, 0, size.0 as i32, size.1 as i32);
        true
    }
}

impl Drop for ScrollPopup {
    /// 【滚动截图】【父视区回收】无参数，释放透明父窗口的视区，子角色与父角色随后由各自所有者回收，无返回值
    fn drop(&mut self) {
        if let Some(viewport) = &self.parent_viewport {
            viewport.destroy();
        }
    }
}
