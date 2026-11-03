pub mod freedesktop;
mod temp;

use log::info;
use std::error::Error;
use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;

use crate::backend::{Notifier, initialize_notifier};

/// duration to wait for the desktop notification daemon via DBus.
const TIMEOUT: Duration = Duration::from_millis(500);

pub const NO_NOTIFY_ARG: &str = "--nonotify";

pub struct Notification<'a> {
    pub summary: &'static str,
    pub body: String,
    pub error: bool,
    pub file: Option<PathBuf>,
    pub png: Option<&'a [u8]>,
}

pub enum Notice<'a> {
    Saved(PathBuf),
    /// Contains `Some(PathBuf)`
    /// if the copied image was also saved to disk,
    /// otherwise the PNG itself, previewed if notif daemon can show it.
    Copied(Option<PathBuf>, Option<&'a [u8]>),
    SaveFailed(String),
    CopyFailed(String),
    PinFailed(String),
    StdoutFailed(String),
    Failed(String),
    TextCopied(String),
    TextInPrimary(String),
    NoText,
    TextFailed(String),
    TextCopyFailed(String),
}

impl<'a> Notice<'a> {
    pub fn spec(self) -> Notification<'a> {
        let mut png = None;
        let (summary, body, error, file) = match self {
            Notice::Saved(path) => (
                "Screenshot saved",
                path.display().to_string(),
                false,
                Some(path),
            ),
            Notice::Copied(Some(path), _) => (
                "Screenshot copied",
                format!("Also saved to {}", path.display()),
                false,
                Some(path),
            ),
            Notice::Copied(None, shot) => {
                png = shot;
                ("Screenshot copied", String::new(), false, None)
            }
            Notice::SaveFailed(e) => ("Couldn't save the screenshot", e, true, None),
            Notice::CopyFailed(e) => ("Couldn't copy the screenshot", e, true, None),
            Notice::PinFailed(e) => ("Couldn't pin the screenshot", e, true, None),
            Notice::StdoutFailed(e) => ("Couldn't write to stdout", e, true, None),
            Notice::Failed(e) => ("Screenshot failed", e, true, None),
            Notice::TextCopied(text) => ("Text copied", preview(&text), false, None),
            Notice::TextInPrimary(text) => (
                "Text ready to paste with the middle click",
                preview(&text),
                false,
                None,
            ),
            Notice::NoText => ("No text found", String::new(), false, None),
            Notice::TextFailed(e) => ("Couldn't read the text", e, true, None),
            Notice::TextCopyFailed(e) => ("Couldn't copy the text", e, true, None),
        };
        Notification {
            summary,
            body,
            error,
            file,
            png,
        }
    }
}

fn preview(text: &str) -> String {
    const MAX: usize = 200;
    match text.char_indices().nth(MAX) {
        Some((end, _)) => format!("{}…", &text[..end]),
        None => text.to_string(),
    }
}

pub struct StderrNotifier;

#[async_trait]
impl Notifier for StderrNotifier {
    async fn notify(&self, n: &Notification<'_>) -> Result<(), Box<dyn Error + Send + Sync>> {
        eprintln!("{}: {}", n.summary, n.body);
        Ok(())
    }
}

pub async fn send(notice: Notice<'_>) {
    if !crate::config::get().notifications.enabled {
        return;
    }
    let n = notice.spec();
    let sent = tokio::time::timeout(TIMEOUT, initialize_notifier().notify(&n)).await;
    let reason = match sent {
        Ok(Ok(())) => return,
        Ok(Err(e)) => e.to_string(),
        Err(_) => "timed out".to_string(),
    };
    info!("notification failed: {reason}");
    let _ = StderrNotifier.notify(&n).await;
}

/// Dispatches notifications synchronously for non Tokio worker processes
/// Panics if invoked inside an active Tokio context. Avoids `zbus::block_on` to prevent
/// spawning per-core threads.
pub fn send_blocking(notice: Notice<'_>) {
    if !crate::config::get().notifications.enabled {
        return;
    }
    match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt.block_on(send(notice)),
        Err(_) => {
            let n = notice.spec();
            eprintln!("{}: {}", n.summary, n.body);
        }
    }
}
