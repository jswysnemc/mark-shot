//! 【全局快捷键】【Wayland门户】系统决定最终按键，不通过Xwayland模拟全局注册

use super::super::{Action, Reporter};
use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
use futures::StreamExt;

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

/// 【全局快捷键】【门户会话】按shortcut请求一次系统授权，reporter交付实际绑定及触发，连接销毁自动关闭会话
pub(super) async fn run(shortcut: &str, reporter: &Reporter) -> Result<(), String> {
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
        let response = portal
            .bind_shortcuts(
                &session,
                &[NewShortcut::new("screenshot", "Mark Shot — Screenshot")
                    .preferred_trigger(preferred.as_str())],
                None,
                Default::default(),
            )
            .await?
            .response()?;
        let actual = response
            .shortcuts()
            .iter()
            .find(|shortcut| shortcut.id() == "screenshot")
            .ok_or("portal did not bind screenshot")?;
        reporter.status(super::super::status::Registration::Registered(
            actual.trigger_description().into(),
        ));
        loop {
            tokio::select! {
                _ = closed.next() => return Err("global shortcut session closed by the desktop".into()),
                event = activated.next() => {
                    let Some(event) = event else { break; };
                    if event.shortcut_id() == "screenshot" {
                        let token = event.options().get("activation_token")
                            .and_then(|value| <&str>::try_from(value).ok()).map(str::to_owned);
                        reporter.activated(Action::Capture, token);
                    }
                }
                event = changed.next() => {
                    let Some(event) = event else { break; };
                    if let Some(shortcut) = event.shortcuts().iter().find(|shortcut| shortcut.id() == "screenshot") {
                        reporter.status(super::super::status::Registration::Registered(shortcut.trigger_description().into()));
                    } else {
                        return Err("screenshot shortcut removed by the desktop".into());
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
