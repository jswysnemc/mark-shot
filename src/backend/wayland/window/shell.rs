//! 【原生窗口】【窗口协议】响应配置、整数缩放与帧回调

use smithay_client_toolkit::{
    compositor::CompositorHandler,
    output::{OutputHandler, OutputState},
    shell::{
        WaylandSurface,
        xdg::{
            XdgSurface,
            window::{Window, WindowConfigure, WindowHandler},
        },
    },
};
use wayland_client::{
    Connection, QueueHandle,
    protocol::{wl_output, wl_surface},
};

use super::{WindowEvent, runtime::NativeWindow};

impl WindowHandler for NativeWindow {
    /// 【原生窗口】【关闭请求】响应合成器对 window 的关闭请求；设置退出状态
    fn request_close(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &Window) {
        self.exit = true;
    }

    /// 【原生窗口】【窗口配置】应用 configure 的大小与边界；通知内容并安排重绘
    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &Window,
        configure: WindowConfigure,
        _: u32,
    ) {
        let bounds = configure.suggested_bounds.unwrap_or(self.size);
        self.size = (
            configure
                .new_size
                .0
                .map_or(self.size.0.min(bounds.0), |width| width.get())
                .max(1),
            configure
                .new_size
                .1
                .map_or(self.size.1.min(bounds.1), |height| height.get())
                .max(1),
        );
        if let Some(viewport) = &self.viewport {
            viewport.set_destination(self.size.0 as i32, self.size.1 as i32);
        }
        self.window
            .set_window_geometry(0, 0, self.size.0, self.size.1);
        self.configured = true;
        self.dirty = true;
        self.dispatch(WindowEvent::Resize { size: self.size }, None);
    }
}

impl CompositorHandler for NativeWindow {
    /// 【原生窗口】【整数缩放】无分数缩放协议时应用 factor；重新绘制物理缓冲
    fn scale_factor_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        surface: &wl_surface::WlSurface,
        factor: i32,
    ) {
        if surface == self.window.wl_surface() && self.fractional.is_none() {
            self.scale = factor.max(1) as f32;
            if self.viewport.is_none() {
                surface.set_buffer_scale(factor.max(1));
            }
            self.dirty = true;
        }
    }

    /// 【原生窗口】【变换通知】普通窗口保持逻辑坐标方向；忽略输出 transform
    fn transform_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: wl_output::Transform,
    ) {
    }

    /// 【原生窗口】【帧回调】收到当前 surface 的帧许可后解除提交限制；不启动定时刷新
    fn frame(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        surface: &wl_surface::WlSurface,
        _: u32,
    ) {
        if surface == self.window.wl_surface() {
            self.frame_pending = false;
        }
    }

    /// 【原生窗口】【进入输出】输出缩放由合成器和分数缩放协议维护；不修改内容
    fn surface_enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }

    /// 【原生窗口】【离开输出】输出离开不销毁普通窗口；后续配置更新缩放
    fn surface_leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
}

impl OutputHandler for NativeWindow {
    /// 【原生窗口】【输出状态】返回运行时维护的输出信息
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.outputs
    }

    /// 【原生窗口】【新增输出】输出绑定由 OutputState 处理；窗口等待配置事件
    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}

    /// 【原生窗口】【输出更新】输出信息由 OutputState 更新；无额外操作
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}

    /// 【原生窗口】【输出移除】窗口迁移由合成器处理；不关闭应用
    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}
