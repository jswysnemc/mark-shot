//! 【设置窗口】【关于页面】展示版本和当前会话实际可识别的桌面信息

use super::{layout::rect, widgets};
use crate::{
    renderer::{Painter, text::TextStyle},
    theme::window::Palette,
    ui::controls,
};
use tiny_skia::Rect;

/// 【应用信息】【环境读取】按 keys 顺序读取非空环境变量，返回值或明确的未识别提示
fn environment(keys: &[&str]) -> String {
    keys.iter()
        .find_map(|key| {
            std::env::var(key)
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .unwrap_or_else(|| "未识别 / Unknown".into())
}

/// 【应用信息】【窗口管理器】无参数，返回已检测的合成器或 X11 窗口管理器名称
fn window_manager() -> String {
    #[cfg(target_os = "windows")]
    {
        "Desktop Window Manager (DWM)".into()
    }
    #[cfg(target_os = "linux")]
    {
        for (key, name) in [
            ("NIRI_SOCKET", "niri"),
            ("HYPRLAND_INSTANCE_SIGNATURE", "Hyprland"),
            ("SWAYSOCK", "Sway"),
        ] {
            if std::env::var_os(key).is_some() {
                return name.into();
            }
        }
        if std::env::var_os("WAYLAND_DISPLAY").is_none()
            && let Some(name) = x11_wm()
        {
            return name;
        }
        wayland_wm().unwrap_or_else(|| "未识别 / Unknown".into())
    }
}

/// 【应用信息】【Wayland管理器】无参数，从当前显示套接字的内核对端识别合成器进程，无法访问时返回空值
#[cfg(target_os = "linux")]
fn wayland_wm() -> Option<String> {
    use std::os::unix::net::UnixStream;
    let display = std::path::PathBuf::from(std::env::var_os("WAYLAND_DISPLAY")?);
    let path = if display.is_absolute() {
        display
    } else {
        std::path::PathBuf::from(std::env::var_os("XDG_RUNTIME_DIR")?).join(display)
    };
    let stream = UnixStream::connect(path).ok()?;
    let peer =
        nix::sys::socket::getsockopt(&stream, nix::sys::socket::sockopt::PeerCredentials).ok()?;
    let name = std::fs::read_to_string(format!("/proc/{}/comm", peer.pid())).ok()?;
    let name = name.trim();
    Some(if name == "gnome-shell" {
        "gnome-shell (Mutter)".into()
    } else {
        name.into()
    })
}

/// 【应用信息】【X11管理器】无参数，读取 EWMH 管理器名称；不支持协议时返回空值
#[cfg(target_os = "linux")]
fn x11_wm() -> Option<String> {
    use x11rb::{
        connection::Connection,
        protocol::xproto::{AtomEnum, ConnectionExt},
    };
    let (connection, screen) = x11rb::connect(None).ok()?;
    let atom = connection
        .intern_atom(false, b"_NET_SUPPORTING_WM_CHECK")
        .ok()?
        .reply()
        .ok()?
        .atom;
    let reply = connection
        .get_property(
            false,
            connection.setup().roots[screen].root,
            atom,
            AtomEnum::WINDOW,
            0,
            1,
        )
        .ok()?
        .reply()
        .ok()?;
    let window = reply.value32()?.next()?;
    let name = connection
        .intern_atom(false, b"_NET_WM_NAME")
        .ok()?
        .reply()
        .ok()?
        .atom;
    let value = connection
        .get_property(false, window, name, AtomEnum::ANY, 0, 256)
        .ok()?
        .reply()
        .ok()?
        .value;
    String::from_utf8(value)
        .ok()
        .filter(|value| !value.is_empty())
}

pub(super) struct Information {
    desktop: String,
    wm: String,
    protocol: String,
}

impl Information {
    /// 【应用信息】【会话检测】无参数，创建一次环境快照，重绘不重复建立系统连接
    pub fn detect() -> Self {
        Self {
            desktop: if cfg!(target_os = "windows") {
                "Windows Desktop".into()
            } else {
                environment(&[
                    "XDG_CURRENT_DESKTOP",
                    "XDG_SESSION_DESKTOP",
                    "DESKTOP_SESSION",
                ])
            },
            wm: window_manager(),
            protocol: if cfg!(target_os = "windows") {
                "Win32".into()
            } else if std::env::var_os("WAYLAND_DISPLAY").is_some() {
                "Wayland".into()
            } else {
                "X11".into()
            },
        }
    }
}

/// 【应用信息】【页面绘制】按 painter、viewport 和 chinese 绘制紧凑信息行，无返回值
pub(super) fn draw(painter: &mut Painter<'_>, viewport: Rect, chinese: bool, info: &Information) {
    let palette = Palette::current();
    let rows = [
        (
            if chinese { "版本" } else { "Version" },
            env!("CARGO_PKG_VERSION").into(),
        ),
        (
            if chinese { "操作系统" } else { "OS" },
            format!("{} / {}", std::env::consts::OS, std::env::consts::ARCH),
        ),
        (
            if chinese { "桌面环境" } else { "Desktop" },
            info.desktop.clone(),
        ),
        (
            if chinese {
                "窗口管理器 / 合成器"
            } else {
                "WM / compositor"
            },
            info.wm.clone(),
        ),
        (
            if chinese {
                "显示协议"
            } else {
                "Display protocol"
            },
            info.protocol.clone(),
        ),
    ];
    let area = rect(
        viewport.left(),
        viewport.top(),
        viewport.width() - 14.0,
        288.0,
    );
    widgets::panel(painter, area, "Mark Shot");
    for (index, (label, value)) in rows.iter().enumerate() {
        let y = area.top() + 42.0 + index as f32 * 39.0;
        let label_width = if area.width() < 480.0 { 138.0 } else { 168.0 };
        painter.text(
            rect(area.left() + 16.0, y, label_width, 28.0),
            label,
            TextStyle::new(11.0, palette.muted),
        );
        let value = controls::fit_label(painter, value, 12.0, area.width() - label_width - 40.0);
        painter.text(
            rect(
                area.left() + label_width + 24.0,
                y,
                area.width() - label_width - 40.0,
                28.0,
            ),
            &value,
            TextStyle::new(12.0, palette.text),
        );
    }
    painter.text(
        rect(
            area.left() + 16.0,
            area.bottom() - 37.0,
            area.width() - 32.0,
            26.0,
        ),
        "github.com/jswysnemc/mark-shot",
        TextStyle::new(11.0, palette.muted),
    );
}
