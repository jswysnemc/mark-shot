mod capture_chain;
pub mod notify;
pub mod output_selection;
pub mod wayland;

use crate::types::{Capture, CaptureResult, CursorIcon, DamageRect, Output, OverlayEvent};
use async_trait::async_trait;
use log::info;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use wayland_client::Connection;

pub const FRAME_FALLBACK: Duration = Duration::from_millis(150);

#[async_trait]
pub trait CaptureMethod: Send + Sync {
    async fn capture_frame(
        &self,
        outputs: &[Output],
    ) -> Result<CaptureResult, Box<dyn std::error::Error>>;

    async fn prepare(&self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    /// 【Wayland】【窗口捕获】后端未提供精确窗口捕获时返回错误，不替换为整屏图片
    async fn capture_active_window(&self) -> Result<Capture, Box<dyn std::error::Error>> {
        Err("capturing the active window is only supported on KDE Plasma".into())
    }
}

/// 【Wayland】【捕获初始化】为普通截图选择原生后端；默认不捕获鼠标指针
pub fn initialize_capture(conn: &Connection) -> Box<dyn CaptureMethod> {
    initialize_capture_with_cursor(conn, false)
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

pub trait ClipboardProvider {
    fn copy_image_to_clipboard(&self, png_data: &[u8]) -> Result<(), Box<dyn std::error::Error>>;
}

pub trait ScreenOverlay: Send {
    /// 【输入法】【状态同步】把 snapshot 交给原生协议，空值停用正文输入；默认保留普通键盘后端，返回协议结果
    fn set_text_input(
        &mut self,
        _snapshot: Option<&crate::types::text_input::Snapshot>,
    ) -> crate::AppResult {
        Ok(())
    }
    /// 【原生弹层】【帧同步】按 frame 打开或更新菜单，空值关闭；返回是否提供原生弹层，默认实现供无表面的测试后端使用
    fn set_popup(
        &mut self,
        _frame: Option<crate::types::popup::PopupFrame<'_>>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(false)
    }
    /// 【截图覆盖层】【暂停显示】隐藏全部截图表面并释放输入抓取；保留原图和输出绑定，返回协议执行结果
    fn suspend(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Err("overlay suspension is unavailable".into())
    }
    /// 【截图覆盖层】【恢复显示】在原输出重新创建表面；调用方恢复冻结背景，返回协议执行结果
    fn resume(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Err("overlay resumption is unavailable".into())
    }
    fn prefault(&mut self);
    fn present(&mut self, targets: &[usize]) -> Result<(), Box<dyn std::error::Error>>;
    fn stage_frame(
        &mut self,
        monitor_idx: usize,
        pixels: &[u8],
        damage: Option<DamageRect>,
    ) -> Result<(), Box<dyn std::error::Error>>;
    fn flush(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    /// `None` waits for the compositor indefinitely, a duration wakes up with
    /// `OverlayEvent::Tick` once it runs out.
    fn next_event(
        &mut self,
        timeout: Option<Duration>,
    ) -> Result<OverlayEvent, Box<dyn std::error::Error>>;
    fn discovered_outputs(&self) -> &[Output];
    fn hidpi(&self) -> bool;
    /// true once compositor has shown the last frame
    /// or after FRAME_FALLBACK without an answer, to avoid monitor freezing
    fn ready(&self, monitor_idx: usize) -> bool;
    fn active_output(&mut self) -> Option<usize>;
    fn retain_outputs(&mut self, keep: &[usize]) -> Result<(), Box<dyn std::error::Error>>;
    fn set_background(
        &mut self,
        monitor_idx: usize,
        pixels: &[u8],
        width: u32,
        height: u32,
    ) -> Result<(), Box<dyn std::error::Error>>;
    fn set_cursor(&mut self, icon: CursorIcon);
    /// Drops the static snapshot subsurface on `monitor_idx` to expose the live screen beneath.
    fn clear_background(&mut self, monitor_idx: usize) -> Result<(), Box<dyn std::error::Error>>;
    /// Sets the input region on `monitor_idx` to `rect`, or allows all pointer events
    /// to fall through to windows beneath when `None`.
    fn set_input_rect(&mut self, monitor_idx: usize, rect: Option<(i32, i32, i32, i32)>);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OverlayShell {
    Auto,
    Window,
    Layer,
}

/// Selection of backend for displaying on the screenshot and interface screen
/// GNOME is not supported and has not been tested yet.
///
/// layer_shell works as a system overlay; it will be on top of everything else, so the screenshot
/// cannot be temporarily moved to another desktop or minimized, which seems non-optimal.
/// Therefore, a forced full-screen window is used as default.
///
/// Niri, and likely other scroll managers, places every new full-screen window on the side
/// making the user experience clunky since every trigger plays an animation of opening a new app.
/// Therefore, layer_shell is used there instead.
///
/// COSMIC gets layer_shell too since a full-screen window there flashes gray while it opens,
/// and there is few focus grabbing issues.
/// Make sense to return to Window if these issues get fixed
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

#[async_trait]
pub trait Notifier {
    async fn notify(
        &self,
        n: &notify::Notification<'_>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

pub fn initialize_notifier() -> Box<dyn Notifier> {
    Box::new(notify::freedesktop::FreedesktopNotifier)
}
