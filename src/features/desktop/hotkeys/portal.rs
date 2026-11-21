//! 【全局快捷键】【Wayland门户】系统决定最终按键，不通过Xwayland模拟全局注册

use super::super::{Action, Reporter};
use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut, Shortcut};
use futures::StreamExt;

#[cfg(test)]
#[path = "portal_tests.rs"]
mod tests;

/// 【全局快捷键】【绑定状态】按 shortcuts 向 reporter 交付截图快捷键状态，返回是否具有实际按键绑定
fn report_binding(shortcuts: &[Shortcut], reporter: &Reporter) -> bool {
    if let Some(shortcut) = shortcuts
        .iter()
        .find(|shortcut| shortcut.id() == "screenshot")
        .filter(|shortcut| !shortcut.trigger_description().trim().is_empty())
    {
        reporter.status(super::super::status::Registration::Registered(
            shortcut.trigger_description().into(),
        ));
        true
    } else {
        reporter.status(super::super::status::Registration::Failed(
            "No screenshot shortcut is assigned; configure it in the desktop's global shortcut settings".into(),
        ));
        false
    }
}

/// 【全局快捷键】【活动窗口绑定】按 shortcuts 和 reporter 校验独立窗口快捷键，返回是否获得系统绑定
fn report_window_binding(shortcuts: &[Shortcut], reporter: &Reporter) -> bool {
    if shortcuts.iter().any(|shortcut| {
        shortcut.id() == "active-window" && !shortcut.trigger_description().trim().is_empty()
    }) {
        true
    } else {
        reporter.status(super::super::status::Registration::Failed(
            "No active-window screenshot shortcut is assigned; configure it in the desktop's global shortcut settings".into(),
        ));
        false
    }
}

/// 【全局快捷键】【动作路由】按 id 返回截图动作，忽略其他门户快捷键，不创建界面
fn action(id: &str) -> Option<Action> {
    match id {
        "screenshot" => Some(Action::Capture),
        "active-window" => Some(Action::CaptureWindow),
        _ => None,
    }
}

/// 【全局快捷键】【门户触发文本】按shortcut转换为XDG快捷键语法，返回规范组合或解析错误
fn trigger(shortcut: &str) -> Result<String, String> {
    let chord = crate::keys::parse(shortcut)?;
    let mut parts = Vec::new();
    for (enabled, text) in [
        (chord.mods.ctrl, "CTRL"),
        (chord.mods.alt, "ALT"),
        (chord.mods.shift, "SHIFT"),
        (chord.mods.logo, "LOGO"),
    ] {
        if enabled {
            parts.push(text.to_owned());
        }
    }
    parts.push(match chord.key {
        crate::keys::Key::PrintScreen => "Print".into(),
        crate::keys::Key::Enter => "Return".into(),
        crate::keys::Key::Space => "space".into(),
        crate::keys::Key::Backspace => "BackSpace".into(),
        crate::keys::Key::PageUp => "Page_Up".into(),
        crate::keys::Key::PageDown => "Page_Down".into(),
        crate::keys::Key::Char(character) => character.to_uppercase().to_string(),
        crate::keys::Key::F(number) => format!("F{number}"),
        key => format!("{key:?}"),
    });
    Ok(parts.join("+"))
}

/// 【全局快捷键】【门户会话】按 shortcut 和可选 window_shortcut 请求一次授权，reporter 交付绑定及触发，返回门户错误
pub(super) async fn run(
    shortcut: &str,
    window_shortcut: Option<&str>,
    reporter: &Reporter,
) -> Result<(), String> {
    super::identity::ensure().map_err(|error| error.to_string())?;
    let result = async {
        let connection = zbus::Connection::session().await?;
        if let Err(error) = super::identity::register(&connection).await
        {
            log::info!("【全局快捷键】【门户身份】主机登记不可用，继续使用系统身份: {error}");
        }
        let portal = GlobalShortcuts::with_connection(connection).await?;
        let session = portal.create_session(Default::default()).await?;
        let mut activated = portal.receive_activated().await?;
        let mut changed = portal.receive_shortcuts_changed().await?;
        let mut closed = session.receive_closed().await?;
        let preferred = trigger(shortcut)?;
        let preferred_window = window_shortcut.map(trigger).transpose()?;
        let mut requested = vec![NewShortcut::new("screenshot", "Mark Shot — Screenshot")
            .preferred_trigger(preferred.as_str())];
        if let Some(preferred) = &preferred_window {
            requested.push(NewShortcut::new("active-window", "Mark Shot — Active window screenshot")
                .preferred_trigger(preferred.as_str()));
        }
        let response = portal
            .bind_shortcuts(
                &session,
                &requested,
                None,
                Default::default(),
            )
            .await?
            .response()?;
        // 1. 【全局快捷键】【空绑定恢复】门户可能保留取消授权后的空绑定，仅首次响应请求系统配置
        let bound = report_binding(response.shortcuts(), reporter);
        let window_bound = window_shortcut.is_none() || report_window_binding(response.shortcuts(), reporter);
        if (!bound || !window_bound) && portal.version() >= 2
            && let Err(error) = portal.configure_shortcuts(&session, None, Default::default()).await
        {
            log::error!("【全局快捷键】【系统配置】无法打开快捷键配置: {error}");
        }
        loop {
            tokio::select! {
                _ = closed.next() => return Err("global shortcut session closed by the desktop".into()),
                event = activated.next() => {
                    let Some(event) = event else { break; };
                    if let Some(action) = action(event.shortcut_id())
                        .filter(|action| *action != Action::CaptureWindow || window_shortcut.is_some()) {
                        let token = event.options().get("activation_token")
                            .and_then(|value| <&str>::try_from(value).ok()).map(str::to_owned);
                        reporter.activated(action, token);
                    }
                }
                event = changed.next() => {
                    let Some(event) = event else { break; };
                    // 2. 【全局快捷键】【绑定更新】保持会话以接收系统配置恢复，后续空绑定不重复弹出配置
                    report_binding(event.shortcuts(), reporter);
                    if window_shortcut.is_some() {
                        report_window_binding(event.shortcuts(), reporter);
                    }
                }
            }
        }
        session.close().await?;
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }
    .await;
    result.map_err(|error| format!("{error}; Wayland requires the GlobalShortcuts portal"))
}
