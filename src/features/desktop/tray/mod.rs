//! 【系统托盘】【平台分发】Linux使用StatusNotifierItem，Windows使用系统通知区域

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "windows")]
mod windows;
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

/// 【系统托盘】【菜单语言】无参数，按当前界面语言返回有序菜单和动作
fn entries() -> [(&'static str, super::Action); 4] {
    use super::Action;
    let labels =
        if crate::ui::language::Language::current() == crate::ui::language::Language::Chinese {
            ["截图", "剪贴板贴图", "设置", "退出"]
        } else {
            ["Screenshot", "Pin clipboard", "Settings", "Quit"]
        };
    [
        (labels[0], Action::Capture),
        (labels[1], Action::Pin),
        (labels[2], Action::Settings),
        (labels[3], Action::Quit),
    ]
}

/// 【系统托盘】【注册生命周期】按reporter和cancelled启动平台托盘，返回前释放图标和菜单
pub(super) async fn run(reporter: Reporter, cancelled: tokio::sync::watch::Receiver<bool>) {
    #[cfg(target_os = "linux")]
    let result = linux::run(&reporter, cancelled).await;
    #[cfg(target_os = "windows")]
    let result = windows::run(&reporter, cancelled).await;
    if let Err(error) = result {
        reporter.status(super::status::Registration::Failed(error));
    }
}
