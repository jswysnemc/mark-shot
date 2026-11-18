//! 【共享内存测试】【角色协议】只实现配置回归需要的窗口、探测层与光标形状全局

use super::{State, global, noop};
use wayland_protocols::{
    wp::cursor_shape::v1::server::wp_cursor_shape_manager_v1,
    xdg::shell::server::{xdg_surface, xdg_toplevel, xdg_wm_base},
};
use wayland_protocols_wlr::layer_shell::v1::server::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};
use wayland_server::{Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource};

#[derive(Default)]
pub(super) struct Shells {
    windows: Vec<(xdg_surface::XdgSurface, xdg_toplevel::XdgToplevel)>,
    layers: Vec<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1>,
    serial: u32,
}

impl Shells {
    /// 【共享内存测试】【窗口事件】按 width 和 height 配置最后创建的窗口，返回值为空
    pub(super) fn configure_window(&mut self, width: i32, height: i32) {
        self.serial += 1;
        let (surface, toplevel) = self.windows.last().unwrap();
        toplevel.configure(width, height, Vec::new());
        surface.configure(self.serial);
    }

    /// 【共享内存测试】【层事件】无参数，配置最后创建的探测层，返回值为空
    pub(super) fn configure_layer(&mut self) {
        self.serial += 1;
        self.layers.last().unwrap().configure(self.serial, 1, 1);
    }
}

/// 【共享内存测试】【角色全局】按 handle 声明窗口、层与光标形状协议，无返回值
pub(super) fn globals(handle: &DisplayHandle) {
    handle.create_global::<State, xdg_wm_base::XdgWmBase, _>(6, ());
    handle.create_global::<State, zwlr_layer_shell_v1::ZwlrLayerShellV1, _>(4, ());
    handle.create_global::<State, wp_cursor_shape_manager_v1::WpCursorShapeManagerV1, _>(1, ());
}

global!(xdg_wm_base::XdgWmBase);
global!(zwlr_layer_shell_v1::ZwlrLayerShellV1);
global!(wp_cursor_shape_manager_v1::WpCursorShapeManagerV1);

impl Dispatch<xdg_wm_base::XdgWmBase, ()> for State {
    /// 【共享内存测试】【窗口角色创建】从 request 初始化 xdg 表面，无返回值
    fn request(
        _: &mut Self,
        _: &Client,
        _: &xdg_wm_base::XdgWmBase,
        request: xdg_wm_base::Request,
        _: &(),
        _: &DisplayHandle,
        data: &mut DataInit<'_, Self>,
    ) {
        if let xdg_wm_base::Request::GetXdgSurface { id, .. } = request {
            data.init(id, ());
        }
    }
}

impl Dispatch<xdg_surface::XdgSurface, ()> for State {
    /// 【共享内存测试】【顶层创建】从 request 初始化顶层并保存配置事件接收方，无返回值
    fn request(
        state: &mut Self,
        _: &Client,
        surface: &xdg_surface::XdgSurface,
        request: xdg_surface::Request,
        _: &(),
        _: &DisplayHandle,
        data: &mut DataInit<'_, Self>,
    ) {
        if let xdg_surface::Request::GetToplevel { id } = request {
            state
                .shells
                .windows
                .push((surface.clone(), data.init(id, ())));
        }
    }
}

impl Dispatch<zwlr_layer_shell_v1::ZwlrLayerShellV1, ()> for State {
    /// 【共享内存测试】【层角色创建】从 request 初始化探测层并保存配置事件接收方，无返回值
    fn request(
        state: &mut Self,
        _: &Client,
        _: &zwlr_layer_shell_v1::ZwlrLayerShellV1,
        request: zwlr_layer_shell_v1::Request,
        _: &(),
        _: &DisplayHandle,
        data: &mut DataInit<'_, Self>,
    ) {
        if let zwlr_layer_shell_v1::Request::GetLayerSurface { id, .. } = request {
            state.shells.layers.push(data.init(id, ()));
        }
    }
}

noop!(xdg_toplevel::XdgToplevel);
noop!(zwlr_layer_surface_v1::ZwlrLayerSurfaceV1);
noop!(wp_cursor_shape_manager_v1::WpCursorShapeManagerV1);
