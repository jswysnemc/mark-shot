//! 【原生窗口】【协议分发】注册共享内存、视区与分数缩放协议处理器

use smithay_client_toolkit::{
    delegate_dispatch2, delegate_registry,
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    shm::{Shm, ShmHandler},
};
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::wp::{
    fractional_scale::v1::client::{wp_fractional_scale_manager_v1, wp_fractional_scale_v1},
    viewporter::client::{wp_viewport, wp_viewporter},
};

use super::runtime::NativeWindow;
use super::surface::SurfaceId;

impl ShmHandler for NativeWindow {
    /// 【原生窗口】【共享内存】返回当前连接使用的共享内存协议状态
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl ProvidesRegistryState for NativeWindow {
    /// 【原生窗口】【全局注册】返回当前连接的全局对象注册表
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry
    }
    registry_handlers![
        smithay_client_toolkit::output::OutputState,
        smithay_client_toolkit::seat::SeatState
    ];
}

impl Dispatch<wp_viewporter::WpViewporter, ()> for NativeWindow {
    /// 【原生窗口】【视区管理】视区管理对象不产生业务事件
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

impl Dispatch<wp_viewport::WpViewport, SurfaceId> for NativeWindow {
    /// 【原生窗口】【视区对象】视区大小随窗口配置更新；无服务器事件
    fn event(
        _: &mut Self,
        _: &wp_viewport::WpViewport,
        _: wp_viewport::Event,
        _: &SurfaceId,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1, ()> for NativeWindow {
    /// 【原生窗口】【缩放管理】缩放管理对象不产生业务事件
    fn event(
        _: &mut Self,
        _: &wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1,
        _: wp_fractional_scale_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wp_fractional_scale_v1::WpFractionalScaleV1, SurfaceId> for NativeWindow {
    /// 【原生窗口】【分数缩放】根据 event 的 120 分之一缩放值更新缓冲比例；不改变逻辑大小
    fn event(
        state: &mut Self,
        proxy: &wp_fractional_scale_v1::WpFractionalScaleV1,
        event: wp_fractional_scale_v1::Event,
        id: &SurfaceId,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wp_fractional_scale_v1::Event::PreferredScale { scale } = event
            && let Some(surface) = state.surface_mut(*id)
            && surface.fractional.as_ref() == Some(proxy)
        {
            surface.scale = (scale as f32 / 120.0).max(0.25);
            surface.dirty = true;
        }
    }
}

delegate_registry!(NativeWindow);
delegate_dispatch2!(NativeWindow);
