//! 【设置窗口】【关于页面】展示版本和当前会话实际可识别的桌面信息

use super::{layout::rect, widgets};
use crate::{
    renderer::{Painter, text::TextStyle},
    theme::window::Palette,
    ui::controls,
};
use tiny_skia::Rect;

const REPOSITORY_URL: &str = "https://github.com/jswysnemc/mark-shot";
const REPOSITORY_LABEL: &str = "github.com/jswysnemc/mark-shot";

/// 【应用信息】【滚动区域】按 state 返回关于页绘制坐标，链接命中与绘制共用滚动偏移
pub(super) fn viewport(state: &super::window::Preferences) -> Rect {
    let viewport = super::layout::viewport(state);
    rect(
        viewport.left(),
        viewport.top() - state.scroll as f32,
        viewport.width(),
        viewport.height(),
    )
}

/// 【应用信息】【链接区域】按 viewport 返回 GitHub 地址的紧凑点击范围，窄窗口限制到面板内
pub(super) fn repository_rect(viewport: Rect) -> Rect {
    rect(
        viewport.left() + 16.0,
        viewport.top() + 251.0,
        (viewport.width() - 46.0).min(220.0),
        26.0,
    )
}

/// 【应用信息】【浏览器打开】无参数，用系统默认浏览器打开固定仓库地址，返回启动结果
pub(super) fn open_repository() -> crate::AppResult {
    #[cfg(target_os = "linux")]
    {
        use std::process::{Command, Stdio};
        // 1. 【应用信息】【浏览器启动】仓库地址作为独立参数交付系统打开程序
        let mut child = Command::new("xdg-open")
            .arg(REPOSITORY_URL)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        // 2. 【应用信息】【进程回收】后台等待退出，避免阻塞设置窗口
        std::thread::spawn(move || {
            if let Err(error) = child.wait() {
                log::error!("【应用信息】【打开仓库】浏览器启动进程回收失败: {error}");
            }
        });
        Ok(())
    }
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::{
            UI::Shell::ShellExecuteW, UI::WindowsAndMessaging::SW_SHOWNORMAL,
        };
        // 1. 【应用信息】【链接编码】按系统接口要求构造带终止符的地址和打开动作
        let url: Vec<_> = REPOSITORY_URL.encode_utf16().chain([0]).collect();
        let action: Vec<_> = "open".encode_utf16().chain([0]).collect();
        // 2. 【应用信息】【浏览器启动】通过系统地址关联选择默认浏览器
        let result = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                action.as_ptr(),
                url.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                SW_SHOWNORMAL,
            )
        };
        // 3. 【应用信息】【结果校验】系统错误码返回给设置窗口的统一状态提示
        if result as isize <= 32 {
            return Err(format!("cannot open repository URL: {}", result as isize).into());
        }
        Ok(())
    }
}

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

/// 【应用信息】【页面绘制】按 painter、viewport 和 chinese 绘制 info 中的紧凑信息行，hovered 控制仓库链接下划线，无返回值
pub(super) fn draw(
    painter: &mut Painter<'_>,
    viewport: Rect,
    chinese: bool,
    info: &Information,
    hovered: bool,
) {
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
    let link = repository_rect(viewport);
    let label = controls::fit_label(painter, REPOSITORY_LABEL, 11.0, link.width());
    painter.text(link, &label, TextStyle::new(11.0, palette.accent));
    if hovered {
        let width = painter.measure(&label, 11.0).min(link.width());
        painter.fill_rounded(
            rect(link.left(), link.top() + 21.0, width, 1.0),
            0.0,
            palette.accent,
        );
    }
}
