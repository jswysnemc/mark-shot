//! 【全局快捷键】【平台分发】按真实会话选择原生X11、系统门户或Windows注册

#[cfg(target_os = "linux")]
mod identity;
#[cfg(target_os = "linux")]
mod portal;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "linux")]
mod x11;
use super::Reporter;

/// 【全局快捷键】【取消边界】按 shortcut 与 active_window_shortcut 注册，reporter 交付结果，cancelled 控制释放，无返回值
pub(super) async fn run(
    shortcut: String,
    active_window_shortcut: String,
    reporter: Reporter,
    cancelled: tokio::sync::watch::Receiver<bool>,
) {
    #[cfg(target_os = "windows")]
    {
        let _ = active_window_shortcut;
        if let Err(error) = windows::run(shortcut, reporter.clone(), cancelled).await {
            reporter.status(super::status::Registration::Failed(error));
        }
    }
    #[cfg(target_os = "linux")]
    let registration = async {
        match crate::backend::session::Session::connect_to_env()
            .map_err(|error| error.to_string())?
        {
            crate::backend::session::Session::X11(display) => {
                x11::run(display, &shortcut, &reporter).await
            }
            crate::backend::session::Session::Wayland(_) => {
                let kde = crate::backend::wayland::app_identity::current("")
                    == crate::backend::wayland::app_identity::APP_ID;
                let window = (kde && !active_window_shortcut.trim().is_empty())
                    .then_some(active_window_shortcut.as_str());
                portal::run(&shortcut, window, &reporter).await
            }
        }
    };
    #[cfg(target_os = "linux")]
    let mut cancelled = cancelled;
    #[cfg(target_os = "linux")]
    tokio::select! {
        result = registration => {
            if let Err(error) = result { reporter.status(super::status::Registration::Failed(error)); }
        }
        _ = cancelled.changed() => {}
    }
}
