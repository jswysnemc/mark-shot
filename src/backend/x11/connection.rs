//! 【X11连接】【显示器发现】共享线程安全连接，按RandR输出查询物理桌面布局

use crate::types::{Output, linux_output::Info};
use std::sync::Arc;
use x11rb::{
    connection::Connection,
    protocol::{randr::ConnectionExt as _, xproto::*},
    xcb_ffi::XCBConnection,
};

#[derive(Clone)]
pub struct Display {
    pub connection: Arc<XCBConnection>,
    pub screen: usize,
}

impl Display {
    /// 【X11连接】【环境连接】无参数，连接DISPLAY并读取默认屏幕，返回认证或连接错误
    pub fn connect() -> crate::AppResult<Self> {
        let (connection, screen) = XCBConnection::connect(None)?;
        Ok(Self {
            connection: Arc::new(connection),
            screen,
        })
    }

    /// 【X11连接】【屏幕描述】无参数，返回连接默认屏幕的资源与像素格式
    pub fn screen(&self) -> &Screen {
        &self.connection.setup().roots[self.screen]
    }

    /// 【X11连接】【属性名称】按name查询或注册原子，返回服务器标识
    pub fn atom(&self, name: &str) -> crate::AppResult<Atom> {
        Ok(self
            .connection
            .intern_atom(false, name.as_bytes())?
            .reply()?
            .atom)
    }

    /// 【X11连接】【整数属性】按window和name读取32位属性，返回有界值列表，缺失属性返回空列表
    pub fn property(&self, window: Window, name: &str) -> crate::AppResult<Vec<u32>> {
        let reply = self
            .connection
            .get_property(false, window, self.atom(name)?, AtomEnum::ANY, 0, 4096)?
            .reply()?;
        Ok(reply
            .value32()
            .map(|values| values.collect())
            .unwrap_or_default())
    }

    /// 【X11连接】【显示器枚举】无参数，返回活动RandR显示器；无扩展时使用默认屏幕
    pub fn outputs(&self) -> crate::AppResult<Vec<Output>> {
        let root = self.screen().root;
        let mut outputs = Vec::new();
        if let Ok(cookie) = self.connection.randr_get_screen_resources_current(root)
            && let Ok(resources) = cookie.reply()
        {
            for id in resources.outputs {
                let output = self
                    .connection
                    .randr_get_output_info(id, resources.config_timestamp)?
                    .reply()?;
                if output.crtc == 0 {
                    continue;
                }
                let crtc = self
                    .connection
                    .randr_get_crtc_info(output.crtc, resources.config_timestamp)?
                    .reply()?;
                if crtc.width == 0 || crtc.height == 0 {
                    continue;
                }
                let position = (i32::from(crtc.x), i32::from(crtc.y));
                let size = (i32::from(crtc.width), i32::from(crtc.height));
                if outputs.iter().any(|item: &Output| {
                    item.info.logical_position == Some(position)
                        && item.info.logical_size == Some(size)
                }) {
                    continue;
                }
                outputs.push(Output {
                    wl_output: None,
                    info: Info::x11(
                        id,
                        String::from_utf8_lossy(&output.name).into_owned(),
                        position,
                        size,
                    ),
                });
            }
        }
        if outputs.is_empty() {
            outputs.push(Output {
                wl_output: None,
                info: Info::x11(
                    root,
                    format!("X11-{}", self.screen),
                    (0, 0),
                    (
                        self.screen().width_in_pixels.into(),
                        self.screen().height_in_pixels.into(),
                    ),
                ),
            });
        }
        Ok(outputs)
    }

    /// 【X11连接】【活动窗口】无参数，返回EWMH活动窗口，缺失时读取键盘焦点，拒绝根窗口
    pub fn active_window(&self) -> crate::AppResult<Window> {
        let root = self.screen().root;
        let window = self
            .property(root, "_NET_ACTIVE_WINDOW")?
            .first()
            .copied()
            .unwrap_or(self.connection.get_input_focus()?.reply()?.focus);
        if window <= 1 || window == root {
            return Err("no active X11 application window".into());
        }
        Ok(window)
    }
}
