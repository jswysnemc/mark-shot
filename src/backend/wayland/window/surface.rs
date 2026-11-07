//! 【原生窗口】【表面状态】主窗口与子窗口分别维护配置、缩放和缓冲，复用相同提交路径

use super::runtime::NativeWindow;
use super::{WindowContent, WindowSpec, role::Role};
use crate::AppResult;
use smithay_client_toolkit::{
    compositor::{CompositorState, FrameCallbackData},
    shell::xdg::XdgShell,
    shm::slot::{Buffer, SlotPool},
};
use tiny_skia::Pixmap;
use wayland_client::{
    QueueHandle,
    protocol::{wl_shm, wl_surface},
};
use wayland_protocols::wp::{
    fractional_scale::v1::client::{wp_fractional_scale_manager_v1, wp_fractional_scale_v1},
    viewporter::client::{wp_viewport, wp_viewporter},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SurfaceId {
    Main,
    Dialog(u64),
}

pub(super) struct Surface {
    pub role: Role,
    pub spec: WindowSpec,
    pub viewport: Option<wp_viewport::WpViewport>,
    pub fractional: Option<wp_fractional_scale_v1::WpFractionalScaleV1>,
    pub size: (u32, u32),
    pub scale: f32,
    pub dirty: bool,
    pub configured: bool,
    pub frame_pending: bool,
    buffers: Vec<Buffer>,
    canvas: Option<Pixmap>,
    mapped: bool,
}

impl Surface {
    /// 【原生窗口】【表面构造】用 spec、id 和已绑定协议建立独立窗口，qh 提供事件路由，返回未提交表面
    pub fn new(
        spec: WindowSpec,
        id: SurfaceId,
        compositor: &CompositorState,
        shell: &XdgShell,
        viewporter: Option<&wp_viewporter::WpViewporter>,
        fractional_manager: Option<&wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1>,
        qh: &QueueHandle<NativeWindow>,
    ) -> Self {
        let role = Role::window(&spec, compositor, shell, qh);
        Self::from_role(spec, id, role, viewporter, fractional_manager, qh)
    }

    /// 【原生窗口】【角色表面】用 spec、id、role 及缩放资源建立独立缓冲状态，返回未提交表面，不重建业务内容
    pub fn from_role(
        spec: WindowSpec,
        id: SurfaceId,
        role: Role,
        viewporter: Option<&wp_viewporter::WpViewporter>,
        fractional_manager: Option<&wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1>,
        qh: &QueueHandle<NativeWindow>,
    ) -> Self {
        let viewport = viewporter.map(|manager| manager.get_viewport(role.wl_surface(), qh, id));
        let fractional = viewport.as_ref().and_then(|_| {
            fractional_manager
                .map(|manager| manager.get_fractional_scale(role.wl_surface(), qh, id))
        });
        Self {
            size: spec.size,
            spec,
            role,
            viewport,
            fractional,
            scale: 1.0,
            dirty: true,
            configured: false,
            frame_pending: false,
            buffers: Vec::new(),
            canvas: None,
            mapped: false,
        }
    }

    /// 【原生窗口】【角色替换】用 role、size 和缩放资源替换当前协议状态，保留逻辑首帧通知资格，释放旧缓冲及附属协议，无返回值
    pub fn recreate(
        &mut self,
        role: Role,
        size: (u32, u32),
        viewporter: Option<&wp_viewporter::WpViewporter>,
        fractional_manager: Option<&wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1>,
        qh: &QueueHandle<NativeWindow>,
    ) {
        let mut spec = self.spec.clone();
        spec.size = size;
        let mut next = Self::from_role(
            spec,
            SurfaceId::Main,
            role,
            viewporter,
            fractional_manager,
            qh,
        );
        next.mapped = self.mapped;
        *self = next;
    }

    /// 【原生窗口】【表面匹配】返回 surface 是否属于本窗口，比较完整协议对象身份，不使用可复用数字编号
    pub fn matches(&self, surface: &wl_surface::WlSurface) -> bool {
        surface == self.role.wl_surface()
    }

    /// 【原生窗口】【独立帧提交】把 content 绘制到本表面，pool 复用共享内存分配，qh 接收对应帧回调，返回提交错误
    pub fn render(
        &mut self,
        content: &mut dyn WindowContent,
        pool: &mut SlotPool,
        qh: &QueueHandle<NativeWindow>,
    ) -> AppResult {
        if !self.configured || !self.dirty || self.frame_pending {
            return Ok(());
        }
        let width = (self.size.0 as f32 * self.scale).ceil() as i32;
        let height = (self.size.1 as f32 * self.scale).ceil() as i32;
        let pixels = i64::from(width) * i64::from(height);
        if width <= 0 || height <= 0 || pixels > 64 * 1024 * 1024 {
            return Err("window buffer dimensions exceed the supported limit".into());
        }
        let fresh = self.canvas.as_ref().is_none_or(|canvas| {
            canvas.width() != width as u32 || canvas.height() != height as u32
        });
        if fresh {
            self.canvas = Pixmap::new(width as u32, height as u32);
            self.buffers
                .retain(|buffer| buffer.height() == height && buffer.stride() == width * 4);
        }
        let canvas = self
            .canvas
            .as_mut()
            .ok_or("cannot allocate a window canvas")?;
        content.draw(canvas, self.scale);
        let available = self
            .buffers
            .iter()
            .position(|buffer| buffer.canvas(pool).is_some());
        let index = if let Some(index) = available {
            index
        } else {
            if self.buffers.len() >= 3 {
                return Ok(());
            }
            let (buffer, _) =
                pool.create_buffer(width, height, width * 4, wl_shm::Format::Argb8888)?;
            self.buffers.push(buffer);
            self.buffers.len() - 1
        };
        let buffer = &self.buffers[index];
        let bytes = buffer.canvas(pool).ok_or("window buffer is still active")?;
        crate::utils::copy_swizzled(bytes, canvas.data());
        buffer.attach_to(self.role.wl_surface())?;
        self.role.wl_surface().damage_buffer(0, 0, width, height);
        self.role
            .wl_surface()
            .frame(qh, FrameCallbackData(self.role.wl_surface().clone()));
        self.role.commit();
        if !self.mapped {
            content.mapped();
            self.mapped = true;
        }
        self.frame_pending = true;
        self.dirty = false;
        Ok(())
    }
}

impl Drop for Surface {
    /// 【原生窗口】【表面回收】窗口关闭时销毁附属缩放和视区对象，避免重复开关对话框遗留服务器资源，无参数、无返回值
    fn drop(&mut self) {
        if let Some(fractional) = self.fractional.take() {
            fractional.destroy();
        }
        if let Some(viewport) = self.viewport.take() {
            viewport.destroy();
        }
    }
}
