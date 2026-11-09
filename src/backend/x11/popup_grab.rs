//! 【X11菜单】【输入捕获】菜单存活期间接收外部点击，销毁时释放指针和键盘

use super::connection::Display;
use x11rb::{connection::Connection, protocol::xproto::*};

pub(super) struct Grab(Display);

impl Grab {
    /// 【X11菜单】【捕获创建】按display与window捕获输入，失败回收已取得资源，返回捕获句柄
    pub fn new(display: Display, window: Window) -> crate::AppResult<Self> {
        let pointer = display
            .connection
            .grab_pointer(
                true,
                window,
                EventMask::BUTTON_PRESS | EventMask::BUTTON_RELEASE | EventMask::POINTER_MOTION,
                GrabMode::ASYNC,
                GrabMode::ASYNC,
                x11rb::NONE,
                x11rb::NONE,
                x11rb::CURRENT_TIME,
            )?
            .reply()?;
        if pointer.status != GrabStatus::SUCCESS {
            return Err("X11 popup pointer grab unavailable".into());
        }
        let grab = Self(display);
        let keyboard = grab
            .0
            .connection
            .grab_keyboard(
                false,
                window,
                x11rb::CURRENT_TIME,
                GrabMode::ASYNC,
                GrabMode::ASYNC,
            )?
            .reply()?;
        if keyboard.status != GrabStatus::SUCCESS {
            return Err("X11 popup keyboard grab unavailable".into());
        }
        Ok(grab)
    }
}

impl Drop for Grab {
    /// 【X11菜单】【捕获释放】无参数，仅释放本连接输入捕获，不改变用户桌面焦点
    fn drop(&mut self) {
        let _ = self.0.connection.ungrab_pointer(x11rb::CURRENT_TIME);
        let _ = self.0.connection.ungrab_keyboard(x11rb::CURRENT_TIME);
        let _ = self.0.connection.flush();
    }
}
