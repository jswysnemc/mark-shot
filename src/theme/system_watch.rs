//! 【界面主题】【外观订阅】原生设置窗口存活期间接收系统配色变化，关闭后取消监听

pub(crate) struct SystemWatch {
    cancel: Option<tokio::sync::oneshot::Sender<()>>,
}

impl Drop for SystemWatch {
    /// 【界面主题】【监听清理】无参数，通知异步监听退出并释放桌面连接，不阻塞窗口关闭
    fn drop(&mut self) {
        if let Some(cancel) = self.cancel.take() {
            let _ = cancel.send(());
        }
    }
}

/// 【界面主题】【窗口订阅】按 wake 创建系统外观监听，返回随窗口释放的取消句柄
pub(crate) fn watch(wake: crate::backend::window::WindowWake) -> SystemWatch {
    let (cancel, receive) = tokio::sync::oneshot::channel();
    #[cfg(target_os = "linux")]
    std::thread::spawn(move || {
        let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        else {
            return;
        };
        runtime.block_on(async {
            tokio::select! {
                _ = receive => {},
                _ = listen(wake) => {},
            }
        });
    });
    #[cfg(target_os = "windows")]
    let _ = (wake, receive);
    SystemWatch {
        cancel: Some(cancel),
    }
}

/// 【界面主题】【门户事件】按 wake 订阅 Linux 桌面通知，变化时更新共享配色并唤醒窗口
#[cfg(target_os = "linux")]
async fn listen(wake: crate::backend::window::WindowWake) {
    use ashpd::desktop::settings::{ColorScheme, Settings};
    use futures::StreamExt;
    let Ok(settings) = Settings::new().await else {
        return;
    };
    let Ok(mut changes) = settings.receive_color_scheme_changed().await else {
        return;
    };
    if let Ok(scheme) = settings.color_scheme().await {
        super::system::update(if scheme == ColorScheme::PreferDark {
            super::Mode::Dark
        } else {
            super::Mode::Light
        });
        wake.notify();
    }
    while let Some(scheme) = changes.next().await {
        if super::system::update(if scheme == ColorScheme::PreferDark {
            super::Mode::Dark
        } else {
            super::Mode::Light
        }) {
            wake.notify();
        }
    }
}
