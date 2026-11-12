//! 【X11图片窗口】【窗口管理】使用 EWMH 原生管理请求，设置自绘边框及工具栏最小宽度

use super::Overlay;
use crate::{backend::window::WindowEdge, types::image_window::Action};
use x11rb::{connection::Connection, protocol::xproto::*, wrapper::ConnectionExt as _};

impl Overlay {
    /// 【X11图片窗口】【窗口声明】无参数，为当前图片窗口设置标题、客户端装饰和最小尺寸，返回协议结果
    pub(super) fn prepare_image_window(&self) -> crate::AppResult {
        let surface = &self.surfaces.last().ok_or("image window is unavailable")?.1;
        surface.title("Mark Shot - Edit")?;
        let display = &self.display;
        let mut hints = [0u32; 18];
        hints[0] = 1 << 4;
        hints[5] = self.image_minimum.0;
        hints[6] = self.image_minimum.1;
        display.connection.change_property32(
            PropMode::REPLACE,
            surface.window,
            AtomEnum::WM_NORMAL_HINTS,
            AtomEnum::WM_SIZE_HINTS,
            &hints,
        )?;
        let motif = display.atom("_MOTIF_WM_HINTS")?;
        display.connection.change_property32(
            PropMode::REPLACE,
            surface.window,
            motif,
            motif,
            &[2, 0, 0, 0, 0],
        )?;
        Ok(())
    }

    /// 【X11图片窗口】【原生操作】按 action 请求移动、边缘调整、最小化或切换最大化，返回系统协议错误
    pub(super) fn image_action(&mut self, action: Action) -> crate::AppResult {
        if self.image_size.is_none() {
            return Err("image window is unavailable".into());
        }
        let surface = &self
            .surfaces
            .first()
            .ok_or("image window is unavailable")?
            .1;
        let display = &self.display;
        let (message, data) = match action {
            Action::Minimize => (display.atom("WM_CHANGE_STATE")?, [3, 0, 0, 0, 0]),
            Action::ToggleMaximize => (
                display.atom("_NET_WM_STATE")?,
                [
                    2,
                    display.atom("_NET_WM_STATE_MAXIMIZED_VERT")?,
                    display.atom("_NET_WM_STATE_MAXIMIZED_HORZ")?,
                    1,
                    0,
                ],
            ),
            Action::Move | Action::Resize(_) => {
                let pointer = display
                    .connection
                    .query_pointer(display.screen().root)?
                    .reply()?;
                let direction = match action {
                    Action::Resize(edge) => match edge {
                        WindowEdge::TopLeft => 0,
                        WindowEdge::Top => 1,
                        WindowEdge::TopRight => 2,
                        WindowEdge::Right => 3,
                        WindowEdge::BottomRight => 4,
                        WindowEdge::Bottom => 5,
                        WindowEdge::BottomLeft => 6,
                        WindowEdge::Left => 7,
                    },
                    _ => 8,
                };
                display.connection.ungrab_pointer(x11rb::CURRENT_TIME)?;
                (
                    display.atom("_NET_WM_MOVERESIZE")?,
                    [
                        pointer.root_x as u32,
                        pointer.root_y as u32,
                        direction,
                        1,
                        1,
                    ],
                )
            }
        };
        display.connection.send_event(
            false,
            display.screen().root,
            EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
            ClientMessageEvent::new(32, surface.window, message, data),
        )?;
        display.connection.flush()?;
        Ok(())
    }
}
