//! 【贴图】【协议处理】窗口配置、输出缩放及设备生命周期，不承载菜单业务

use super::runtime::{Pin, Shell};
use smithay_client_toolkit::seat::SeatState;
use smithay_client_toolkit::{
    compositor::CompositorHandler,
    delegate_dispatch2, delegate_registry,
    output::OutputState,
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    shell::{
        WaylandSurface,
        wlr_layer::{LayerShellHandler, LayerSurface, LayerSurfaceConfigure},
        xdg::window::{Window, WindowConfigure, WindowHandler},
    },
    shm::{Shm, ShmHandler},
};
use wayland_client::{
    Connection, Dispatch, QueueHandle,
    protocol::{wl_output, wl_surface},
};
use wayland_protocols::wp::viewporter::client::{wp_viewport, wp_viewporter};

impl WindowHandler for Pin {
    /// 【贴图】【窗口关闭】收到当前 window 的关闭请求后结束循环
    fn request_close(&mut self, _: &Connection, _: &QueueHandle<Self>, window: &Window) {
        if matches!(&self.shell, Shell::Window(current) if current == window) {
            self.exit = true;
        }
    }
    /// 【贴图】【窗口配置】当前 window 完成配置后允许提交缓冲；旧角色通知不修改状态
    fn configure(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        window: &Window,
        configure: WindowConfigure,
        _: u32,
    ) {
        if matches!(&self.shell, Shell::Window(current) if current == window) {
            let requested = (
                configure
                    .new_size
                    .0
                    .map_or(self.size.0, |value| value.get()),
                configure
                    .new_size
                    .1
                    .map_or(self.size.1, |value| value.get()),
            );
            self.native_size(requested, configure.is_resizing(), qh);
            self.configured = true;
            self.dirty = true;
        }
    }
}
impl LayerShellHandler for Pin {
    /// 【贴图】【置顶表面关闭】当前 layer 被合成器关闭时退出，不影响已替换角色
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, layer: &LayerSurface) {
        if self
            .preview
            .as_ref()
            .is_some_and(|preview| &preview.layer == layer)
        {
            self.preview = None;
            self.dirty = true;
        }
        if matches!(&self.shell, Shell::Layer(current) if current == layer) {
            self.exit = true;
        }
    }
    /// 【贴图】【置顶表面配置】当前 layer 完成配置后安排重绘
    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        layer: &LayerSurface,
        _: LayerSurfaceConfigure,
        _: u32,
    ) {
        if matches!(&self.shell, Shell::Layer(current) if current == layer) {
            self.configured = true;
            self.dirty = true;
        } else if let Some(preview) = &mut self.preview
            && &preview.layer == layer
        {
            preview.configured = true;
            preview.dirty = true;
        }
    }
}
impl CompositorHandler for Pin {
    /// 【贴图】【输出缩放】按 factor 更新当前 surface 的物理缓冲，逻辑大小保持不变
    fn scale_factor_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        surface: &wl_surface::WlSurface,
        factor: i32,
    ) {
        if surface == self.shell.surface() && self.fractional.is_none() {
            self.render_scale = factor.max(1) as f32;
            self.dirty = true;
        } else if let Some(preview) = &mut self.preview
            && surface == preview.layer.wl_surface()
            && preview.fractional.is_none()
        {
            preview.scale = factor.max(1) as f32;
            preview.dirty = true;
        }
    }
    /// 【贴图】【输出变换】图像自身方向独立于显示器变换，不修改原始像素
    fn transform_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: wl_output::Transform,
    ) {
    }
    /// 【贴图】【帧回调】释放当前 surface 的提交许可，不启动空闲刷新计时器
    fn frame(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        surface: &wl_surface::WlSurface,
        _: u32,
    ) {
        if surface == self.shell.surface() {
            self.frame_pending = false;
        } else if let Some(preview) = &mut self.preview
            && surface == preview.layer.wl_surface()
        {
            preview.frame_pending = false;
        }
    }
    /// 【贴图】【表面进入输出】记录协议通知，窗口几何不随输出进入事件重复重建
    fn surface_enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        surface: &wl_surface::WlSurface,
        output: &wl_output::WlOutput,
    ) {
        if surface == self.shell.surface() && matches!(self.shell, Shell::Window(_)) {
            self.output = Some(output.clone());
            if let Some(info) = self.outputs.info(output)
                && let Some((width, height)) = info.logical_size
            {
                self.bounds = (width.max(1) as u32, height.max(1) as u32);
            }
        }
    }
    /// 【贴图】【表面离开输出】输出离开事件不触发图像变换
    fn surface_leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
}
impl ShmHandler for Pin {
    /// 【贴图】【共享内存状态】返回当前共享内存协议状态
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}
impl ProvidesRegistryState for Pin {
    /// 【贴图】【注册表状态】返回当前连接的全局对象注册表
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry
    }
    registry_handlers![OutputState, SeatState];
}
impl Dispatch<wp_viewporter::WpViewporter, ()> for Pin {
    /// 【贴图】【视口管理器事件】视口管理器没有业务事件
    fn event(
        _: &mut Self,
        _: &wp_viewporter::WpViewporter,
        _: wp_viewporter::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
impl Dispatch<wp_viewport::WpViewport, ()> for Pin {
    /// 【贴图】【视口事件】视口对象没有业务事件
    fn event(
        _: &mut Self,
        _: &wp_viewport::WpViewport,
        _: wp_viewport::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
delegate_registry!(Pin);
delegate_dispatch2!(Pin);
