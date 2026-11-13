//! 【系统托盘】【平台分发】Linux使用StatusNotifierItem，Windows使用系统通知区域

#[cfg(target_os = "linux")]
mod linux;
pub(super) mod menu;
pub(super) mod recording;
#[cfg(test)]
mod recording_tests;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
mod windows_menu;
use super::Reporter;

/// 【系统托盘】【共享图标】无参数，将项目SVG栅格化为两种平台共用的32像素图标，返回解码结果
fn icon() -> Result<tiny_skia::Pixmap, String> {
    let tree = usvg::Tree::from_str(
        include_str!("../../../../assets/hicolor/scalable/apps/mark-shot.svg"),
        &usvg::Options::default(),
    )
    .map_err(|error| error.to_string())?;
    let mut image = tiny_skia::Pixmap::new(32, 32).ok_or("cannot allocate tray icon")?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(32.0 / tree.size().width(), 32.0 / tree.size().height()),
        &mut image.as_mut(),
    );
    Ok(image)
}

/// 【系统托盘】【注册生命周期】按 reporter、cancelled 和 menu 启动平台托盘，状态变化刷新菜单，返回前释放资源
pub(super) async fn run(
    reporter: Reporter,
    cancelled: tokio::sync::watch::Receiver<bool>,
    menu: tokio::sync::watch::Receiver<menu::State>,
) {
    #[cfg(target_os = "linux")]
    let result = linux::run(&reporter, cancelled, menu).await;
    #[cfg(target_os = "windows")]
    let result = windows::run(&reporter, cancelled, menu).await;
    if let Err(error) = result {
        reporter.status(super::status::Registration::Failed(error));
    }
}
