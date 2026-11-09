//! 【桌面集成】【窗口激活】首个已提交窗口消费系统交付的激活凭据

use std::sync::atomic::{AtomicBool, Ordering};
use wayland_client::{Dispatch, QueueHandle, globals::GlobalList, protocol::wl_surface::WlSurface};
pub(super) use wayland_protocols::xdg::activation::v1::client::xdg_activation_v1::XdgActivationV1;

/// 【桌面集成】【焦点申请】按globals、queue和surface激活首次呈现的窗口，无令牌或协议时保持默认焦点策略
pub(super) fn activate<State>(globals: &GlobalList, queue: &QueueHandle<State>, surface: &WlSurface)
where
    State: Dispatch<XdgActivationV1, ()> + 'static,
{
    static USED: AtomicBool = AtomicBool::new(false);
    if USED.load(Ordering::Relaxed) {
        return;
    }
    let Ok(token) = std::env::var("XDG_ACTIVATION_TOKEN") else {
        return;
    };
    let Ok(activation) = globals.bind::<XdgActivationV1, _, _>(queue, 1..=1, ()) else {
        return;
    };
    if !USED.swap(true, Ordering::Relaxed) {
        activation.activate(token, surface);
    }
    activation.destroy();
}
