//! 【Linux平台】【工厂入口】保留既有Wayland捕获与表面策略

use super::super::{
    CaptureMethod, ClipboardProvider, Notifier, OverlayShell, ScreenOverlay, capture_chain, notify,
    wayland,
};
use log::info;
use wayland_client::Connection;

/// 【Wayland】【捕获初始化】为普通截图选择原生后端；默认不捕获鼠标指针
pub fn initialize_capture(conn: &Connection) -> Box<dyn CaptureMethod> {
    initialize_capture_with_cursor(conn, false)
}

/// 【Wayland】【窗口后端选择】按 conn、指针与复制授权选择精确窗口后端；返回不使用显示器裁切的捕获器
pub fn initialize_window_capture(
    conn: &Connection,
    include_cursor: bool,
    allow_clipboard: bool,
) -> Box<dyn CaptureMethod> {
    let forced = std::env::var("MARK_SHOT_CAPTURE").unwrap_or_default();
    if matches!(
        forced.as_str(),
        "kde" | "image-copy" | "screencopy" | "portal"
    ) {
        return initialize_capture_with_cursor(conn, include_cursor);
    }
    if !forced.is_empty() && forced != "niri-window" {
        info!("【Wayland】【窗口后端选择】忽略未知后端 {forced}");
    }
    let mut candidates = Vec::new();
    if forced == "niri-window" || std::env::var_os("NIRI_SOCKET").is_some() {
        candidates.push(capture_chain::Candidate {
            name: "niri-window",
            method: Box::new(wayland::capture::niri_window::NiriWindowMethod::new(
                conn.clone(),
                include_cursor,
                allow_clipboard,
            )),
        });
    }
    if forced != "niri-window"
        && std::env::var("XDG_CURRENT_DESKTOP")
            .unwrap_or_default()
            .split(':')
            .any(|desktop| desktop.eq_ignore_ascii_case("KDE"))
    {
        candidates.push(capture_chain::Candidate {
            name: "kde",
            method: capture_method("kde", conn, include_cursor),
        });
    }
    Box::new(capture_chain::CaptureChain::new(candidates))
}

/// 【Wayland】【捕获初始化】根据桌面和协议选择后端；include_cursor 控制是否合成鼠标指针
pub fn initialize_capture_with_cursor(
    conn: &Connection,
    include_cursor: bool,
) -> Box<dyn CaptureMethod> {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    let forced = std::env::var("MARK_SHOT_CAPTURE").unwrap_or_default();
    if matches!(
        forced.as_str(),
        "kde" | "image-copy" | "screencopy" | "portal"
    ) {
        info!("【Wayland】【捕获初始化】强制使用后端 {forced}");
        return capture_method(&forced, conn, include_cursor);
    }
    if !forced.is_empty() {
        info!("【Wayland】【捕获初始化】忽略未知后端 {forced}");
    }
    let mut names = Vec::new();
    if desktop
        .split(':')
        .any(|part| part.eq_ignore_ascii_case("KDE"))
    {
        names.push("kde");
    }
    if wayland::capture::image_copy::supported(conn) {
        names.push("image-copy");
    }
    if wayland::capture::screencopy::supported(conn) {
        names.push("screencopy");
    }
    names.push("portal");
    info!("【Wayland】【捕获初始化】后端顺序: {}", names.join(", "));
    let candidates = names
        .into_iter()
        .map(|name| capture_chain::Candidate {
            name,
            method: capture_method(name, conn, include_cursor),
        })
        .collect();
    Box::new(capture_chain::CaptureChain::new(candidates))
}

/// 【Wayland】【捕获后端】创建 name 对应的协议实现；构造过程不初始化 portal 或读取像素
fn capture_method(name: &str, conn: &Connection, include_cursor: bool) -> Box<dyn CaptureMethod> {
    match name {
        "kde" => Box::new(wayland::capture::kde::KdeMethod::with_cursor(
            include_cursor,
        )),
        "image-copy" => Box::new(wayland::capture::image_copy::ImageCopyMethod::with_cursor(
            conn.clone(),
            include_cursor,
        )),
        "screencopy" => Box::new(wayland::capture::screencopy::ScreencopyMethod::with_cursor(
            conn.clone(),
            include_cursor,
        )),
        _ => Box::new(wayland::capture::portal::PortalMethod { include_cursor }),
    }
}

pub fn initialize_overlay(
    conn: Connection,
) -> Result<Box<dyn ScreenOverlay>, Box<dyn std::error::Error>> {
    let layer = match crate::config::get().general.overlay {
        OverlayShell::Auto => true,
        shell => shell == OverlayShell::Layer,
    };
    info!(
        "overlay surface: {}",
        if layer { "layer" } else { "window" }
    );
    Ok(Box::new(wayland::overlay::WaylandOverlay::new(
        conn, layer,
    )?))
}

pub fn initialize_clipboard() -> Box<dyn ClipboardProvider> {
    Box::new(wayland::clipboard::ext_data_control::ClipboardMethod)
}

pub fn initialize_notifier() -> Box<dyn Notifier> {
    Box::new(notify::freedesktop::FreedesktopNotifier)
}
