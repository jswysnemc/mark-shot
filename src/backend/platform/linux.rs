//! 【Linux平台】【工厂入口】保留既有Wayland捕获与表面策略

use super::super::{
    CaptureMethod, ClipboardProvider, Notifier, OverlayShell, ScreenOverlay, capture_chain, notify,
    wayland,
};
use crate::backend::session::Session;
use log::info;
use wayland_client::Connection;

/// 【Wayland】【捕获初始化】为普通截图选择原生后端；默认不捕获鼠标指针
pub fn initialize_capture(conn: &Session) -> Box<dyn CaptureMethod> {
    initialize_capture_with_cursor(conn, false)
}

/// 【Wayland】【窗口后端选择】按 conn、指针与复制授权选择精确窗口后端；返回不使用显示器裁切的捕获器
pub fn initialize_window_capture(
    conn: &Session,
    include_cursor: bool,
    allow_clipboard: bool,
) -> Box<dyn CaptureMethod> {
    let Session::Wayland(conn) = conn else {
        return initialize_capture_with_cursor(conn, include_cursor);
    };
    let forced = std::env::var("MARK_SHOT_CAPTURE").unwrap_or_default();
    if matches!(
        forced.as_str(),
        "kde" | "image-copy" | "screencopy" | "portal"
    ) {
        return wayland_capture_with_cursor(conn, include_cursor);
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
    if crate::config::preferences::current()
        .capture
        .kde_kwin_screenshot
        && forced != "niri-window"
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
    conn: &Session,
    include_cursor: bool,
) -> Box<dyn CaptureMethod> {
    match conn {
        Session::Wayland(connection) => wayland_capture_with_cursor(connection, include_cursor),
        Session::X11(display) => Box::new(crate::backend::x11::capture::Method {
            display: display.clone(),
            include_cursor,
        }),
    }
}

/// 【Linux捕获】【提前授权】在读取显示器列表之前完成 Portal 授权，返回带挂载授权的捕获后端。
///
/// 用户先开机、再插外接屏或改显示器排列时，合成器要到授权窗口关闭前后才把新布局发过来，
/// 而旧流程先读列表再授权，读到的就可能是上一版布局，绑定阶段会因位置对不上而失败。
/// 提前授权把授权等待和布局稳定合并成一段：授权完成时再读列表，读到的就是当前布局。
/// 原生协议可用（wlroots/niri/KDE 自有链路）或 MARK_SHOT_CAPTURE 强制指定后端时返回 None，
/// 交给普通捕获链，避免多弹一次不必要的授权窗口。
pub async fn prepare_portal_capture(
    conn: &Session,
    include_cursor: bool,
) -> Option<Box<dyn CaptureMethod>> {
    let Session::Wayland(connection) = conn else {
        return None;
    };
    // 1. 【Linux捕获】【后端可用性】只有 Portal 是唯一选择时才提前授权，否则会多弹一次没必要的权限窗口
    if wayland::capture::image_copy::supported(connection)
        || wayland::capture::screencopy::supported(connection)
        || std::env::var_os("NIRI_SOCKET").is_some()
    {
        return None;
    }
    if !std::env::var("MARK_SHOT_CAPTURE")
        .unwrap_or_default()
        .is_empty()
    {
        return None;
    }
    // 2. 【Linux捕获】【KDE例外】KDE 的 Portal 链路由 KWin 自己管理，在多显示器下一直正常，
    //    没必要为它多弹一次权限窗口；只在真正需要 Portal 的桌面（GNOME 等）提前授权
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    if desktop
        .split(':')
        .any(|part| part.eq_ignore_ascii_case("KDE"))
    {
        return None;
    }
    match wayland::capture::portal::authorize(include_cursor).await {
        Ok(capture) => {
            log::info!("【Linux捕获】【提前授权】Portal 授权已就绪，读取显示器列表");
            Some(Box::new(
                wayland::capture::portal::PortalMethod::with_cursor(include_cursor)
                    .with_prepared(capture),
            ))
        }
        Err(error) => {
            // 3. 【Linux捕获】【授权失败】提前授权失败不改变原有行为，按普通链路再试一次，由普通流程决定最终错误
            log::info!("【Linux捕获】【提前授权】跳过: {error}");
            None
        }
    }
}

/// 【Wayland】【捕获分发】按conn和include_cursor选择原生或门户捕获，返回捕获链
fn wayland_capture_with_cursor(conn: &Connection, include_cursor: bool) -> Box<dyn CaptureMethod> {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    if !crate::config::preferences::current()
        .capture
        .kde_kwin_screenshot
        && desktop
            .split(':')
            .any(|part| part.eq_ignore_ascii_case("KDE"))
    {
        return capture_method("portal", conn, include_cursor);
    }
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
        _ => Box::new(wayland::capture::portal::PortalMethod::with_cursor(
            include_cursor,
        )),
    }
}

pub fn initialize_overlay(
    conn: Session,
) -> Result<Box<dyn ScreenOverlay>, Box<dyn std::error::Error>> {
    let conn = match conn {
        Session::X11(display) => {
            return Ok(Box::new(crate::backend::x11::overlay::Overlay::new(
                display,
            )?));
        }
        Session::Wayland(connection) => connection,
    };
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
