//! 【图片编辑】【普通窗口】图片标注使用桌面窗口装饰和原生尺寸协商

use super::state::OverlayState;
use crate::backend::wayland::utils::surface::Role;
use smithay_client_toolkit::shell::xdg::{XdgSurface, window::WindowDecorations};
use wayland_client::{QueueHandle, protocol::wl_surface::WlSurface};

/// 【图片编辑】【窗口创建】以 state、queue、surface 和 size 建立可调整窗口，返回持有其生命周期的角色
pub(super) fn create(
    state: &OverlayState,
    queue: &QueueHandle<OverlayState>,
    surface: WlSurface,
    size: (u32, u32),
) -> Role {
    let window = state
        .xdg_shell
        .create_window(surface, WindowDecorations::ServerDefault, queue);
    window.set_title("Mark Shot — Edit");
    window.set_app_id("mark-shot-editor");
    window.set_min_size(Some((320, 240)));
    window.set_window_geometry(0, 0, size.0, size.1);
    Role::Window(window)
}
