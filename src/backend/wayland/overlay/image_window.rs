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
        .create_window(surface, WindowDecorations::ClientOnly, queue);
    window.set_title("Mark Shot — Edit");
    window.set_app_id("mark-shot-editor");
    window.set_min_size(Some(state.image_minimum));
    window.set_window_geometry(0, 0, size.0, size.1);
    Role::Window(window)
}

/// 【图片窗口】【原生操作】按 state、action 和按下 serial 执行移动、边缘调整及窗口状态变化，返回协议前置条件错误
pub(super) fn action(
    state: &mut OverlayState,
    action: crate::types::image_window::Action,
    serial: Option<u32>,
) -> crate::AppResult {
    use crate::{backend::window::WindowEdge, types::image_window::Action};
    use wayland_protocols::xdg::shell::client::xdg_toplevel::ResizeEdge;
    let window = state
        .surfaces
        .values()
        .find_map(|surface| match &surface.role {
            Role::Window(window) => Some(window),
            _ => None,
        })
        .ok_or("image window is unavailable")?;
    match action {
        Action::Minimize => window.set_minimized(),
        Action::ToggleMaximize => {
            if state.image_maximized {
                window.unset_maximized();
            } else {
                window.set_maximized();
            }
        }
        Action::Move | Action::Resize(_) => {
            let (serial, seat) = state
                .popup_grabs
                .iter()
                .rev()
                .find(|entry| Some(entry.0) == serial)
                .ok_or("window move requires the current pointer press")?;
            if let Action::Resize(edge) = action {
                let edge = match edge {
                    WindowEdge::Top => ResizeEdge::Top,
                    WindowEdge::Bottom => ResizeEdge::Bottom,
                    WindowEdge::Left => ResizeEdge::Left,
                    WindowEdge::Right => ResizeEdge::Right,
                    WindowEdge::TopLeft => ResizeEdge::TopLeft,
                    WindowEdge::TopRight => ResizeEdge::TopRight,
                    WindowEdge::BottomLeft => ResizeEdge::BottomLeft,
                    WindowEdge::BottomRight => ResizeEdge::BottomRight,
                };
                window.resize(seat, *serial, edge);
            } else {
                window.move_(seat, *serial);
            }
        }
    }
    Ok(())
}
