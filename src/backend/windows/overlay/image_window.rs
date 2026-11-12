//! 【Windows图片窗口】【窗口管理】截图覆盖层之外的图片窗口支持标准移动、缩放及任务栏状态

use super::Overlay;
use crate::{backend::window::WindowEdge, types::image_window::Action};
use windows_sys::Win32::UI::{Input::KeyboardAndMouse::ReleaseCapture, WindowsAndMessaging::*};

impl Overlay {
    /// 【Windows图片窗口】【系统操作】按 action 操作唯一图片 HWND，返回窗口不存在错误，不修改标注状态
    pub(super) fn image_action(&mut self, action: Action) -> crate::AppResult {
        if self.image_size.is_none() {
            return Err("image window is unavailable".into());
        }
        let hwnd = self.surface(0)?.handle() as _;
        unsafe {
            match action {
                Action::Minimize => {
                    ShowWindow(hwnd, SW_MINIMIZE);
                }
                Action::ToggleMaximize => {
                    ShowWindow(
                        hwnd,
                        if IsZoomed(hwnd) != 0 {
                            SW_RESTORE
                        } else {
                            SW_MAXIMIZE
                        },
                    );
                }
                Action::Move | Action::Resize(_) => {
                    let command = match action {
                        Action::Resize(edge) => {
                            SC_SIZE
                                | match edge {
                                    WindowEdge::Top => WMSZ_TOP,
                                    WindowEdge::Bottom => WMSZ_BOTTOM,
                                    WindowEdge::Left => WMSZ_LEFT,
                                    WindowEdge::Right => WMSZ_RIGHT,
                                    WindowEdge::TopLeft => WMSZ_TOPLEFT,
                                    WindowEdge::TopRight => WMSZ_TOPRIGHT,
                                    WindowEdge::BottomLeft => WMSZ_BOTTOMLEFT,
                                    WindowEdge::BottomRight => WMSZ_BOTTOMRIGHT,
                                }
                        }
                        _ => SC_MOVE | HTCAPTION,
                    };
                    ReleaseCapture();
                    SendMessageW(hwnd, WM_SYSCOMMAND, command as usize, 0);
                }
            }
        }
        Ok(())
    }
}
