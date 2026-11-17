//! 【Windows平台】【工厂入口】按需创建原生捕获、覆盖层与剪贴板，启动时不加载识别模型

use crate::backend::{
    CaptureMethod, ClipboardProvider, Notifier, ScreenOverlay, session::Session, windows,
};

/// 【Windows平台】【默认捕获】按session创建不含光标的截图后端，返回共享捕获接口
pub fn initialize_capture(session: &Session) -> Box<dyn CaptureMethod> {
    initialize_capture_with_cursor(session, false)
}

/// 【Windows平台】【光标捕获】按session与include_cursor创建原生截图后端，返回共享捕获接口
pub fn initialize_capture_with_cursor(
    _session: &Session,
    include_cursor: bool,
) -> Box<dyn CaptureMethod> {
    Box::new(windows::capture::Method { include_cursor })
}

/// 【Windows平台】【窗口捕获】按session和光标策略创建原生窗口捕获器，allow_clipboard不参与系统截图权限，返回共享接口
pub fn initialize_window_capture(
    session: &Session,
    include_cursor: bool,
    _allow_clipboard: bool,
) -> Box<dyn CaptureMethod> {
    initialize_capture_with_cursor(session, include_cursor)
}

/// 【Windows平台】【编辑覆盖层】按session发现显示器并准备事件源，返回尚未映射的截图后端
pub fn initialize_overlay(_session: Session) -> crate::AppResult<Box<dyn ScreenOverlay>> {
    Ok(Box::new(windows::overlay::Overlay::new()?))
}

/// 【Windows平台】【剪贴板】无参数，返回系统持有数据的剪贴板提供者，不启动常驻进程
pub fn initialize_clipboard() -> Box<dyn ClipboardProvider> {
    Box::new(windows::clipboard::Provider)
}

/// 【Windows平台】【通知】无参数，返回已有标准错误通知提供者，界面内反馈仍使用共享Toast
pub fn initialize_notifier() -> Box<dyn Notifier> {
    Box::new(crate::backend::notify::StderrNotifier)
}
