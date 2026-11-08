// Sends notifications through org.freedesktop.Notifications via D-Bus
use std::collections::HashMap;
use std::error::Error;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use async_trait::async_trait;
use image::RgbaImage;
use log::info;
use zbus::zvariant::Value;
use zbus::{Connection, proxy};

use crate::backend::Notifier;
use crate::backend::notify::{Notification, temp};

const APP_NAME: &str = "mark-shot";
const THUMB: u32 = 256;

#[proxy(
    interface = "org.freedesktop.Notifications",
    default_service = "org.freedesktop.Notifications",
    default_path = "/org/freedesktop/Notifications"
)]
trait Notifications {
    async fn get_capabilities(&self) -> zbus::Result<Vec<String>>;

    async fn get_server_information(&self) -> zbus::Result<(String, String, String, String)>;

    async fn notify(
        &self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: &[&str],
        hints: HashMap<&str, Value<'_>>,
        expire_timeout: i32,
    ) -> zbus::Result<u32>;
}

pub struct FreedesktopNotifier;

/// Pnv previw in the notification
enum Preview {
    /// with png in the icon slot, it gets squiched on kde and looks terrible
    /// so we have to use `x-kde-urls` that gives a full preview with actions
    /// (it works with files)
    KdeUrls,
    /// for now only required and used in swaync
    BodyImage,
    Icon {
        square: bool,
    },
    Skip,
}

impl Preview {
    fn pick(caps: &[String], daemon: &str) -> Self {
        if caps.iter().any(|c| c == "x-kde-urls") {
            Preview::KdeUrls
        } else if daemon == "SwayNotificationCenter" && caps.iter().any(|c| c == "body-images") {
            Preview::BodyImage
        } else if daemon == "cosmic-notifications" {
            // todo: add preview when cosmic add proper image to notifications
            Preview::Skip
        } else {
            Preview::Icon {
                // square is required to prevent daemons from stretching the image, like plasma does
                // mako & dunst keep proportions on their own, square onlly adds empty space and doesn't help
                square: !matches!(daemon, "mako" | "dunst"),
            }
        }
    }
}

#[async_trait]
impl Notifier for FreedesktopNotifier {
    async fn notify(&self, n: &Notification<'_>) -> Result<(), Box<dyn Error + Send + Sync>> {
        let conn = Connection::session().await?;
        let dbus = zbus::fdo::DBusProxy::new(&conn).await?;
        if !dbus
            .name_has_owner("org.freedesktop.Notifications".try_into()?)
            .await?
        {
            return Err("no notification daemon running".into());
        }
        let proxy = NotificationsProxy::new(&conn).await?;

        let caps = proxy.get_capabilities().await?;
        // Escape markup special characters (like `&`) if the daemon supports `body-markup` to prevent parsing errors.
        let markup = caps.iter().any(|c| c == "body-markup");
        let mut body = if markup {
            escape_markup(&n.body)
        } else {
            n.body.clone()
        };

        let mut hints: HashMap<&str, Value> = HashMap::new();
        hints.insert("urgency", Value::U8(1));
        if crate::config::preferences::current().notifications.preview
            && (n.file.is_some() || n.png.is_some())
        {
            let daemon = proxy
                .get_server_information()
                .await
                .map(|(name, ..)| name)
                .unwrap_or_default();
            match Preview::pick(&caps, &daemon) {
                Preview::KdeUrls => {
                    if let Some(file) = shot_file(n) {
                        hints.insert("x-kde-urls", Value::from(vec![file_url(&file)]));
                    }
                }
                Preview::BodyImage => {
                    if let Some(file) = shot_file(n)
                        && let Some(path) = file.to_str()
                        && !path.contains('"')
                    {
                        body.push_str(&format!("<img src=\"{path}\" alt=\"\"/>"));
                    }
                }
                Preview::Icon { square } => {
                    if let Some(file) = &n.file {
                        hints.insert("image-path", Value::from(file_url(file)));
                    } else if let Some(png) = n.png
                        && let Some(image) = image_data(png, square)
                    {
                        hints.insert("image-data", image);
                    }
                }
                Preview::Skip => {}
            }
        }

        proxy
            .notify(APP_NAME, 0, "mark-shot", n.summary, &body, &[], hints, -1)
            .await?;
        Ok(())
    }
}

fn shot_file(n: &Notification<'_>) -> Option<PathBuf> {
    match (&n.file, &n.png) {
        (Some(file), _) => Some(file.clone()),
        (None, Some(png)) => temp::write(png)
            .inspect_err(|e| info!("can't write the preview: {e}"))
            .ok(),
        (None, None) => None,
    }
}

/// 【桌面通知】【图像提示】按 png 和 square 创建有界通知图像，返回 D-Bus 元组，不分配整幅长图
fn image_data(png: &[u8], square: bool) -> Option<Value<'static>> {
    let thumb = thumbnail(png, square)?;
    let (w, h) = (thumb.width() as i32, thumb.height() as i32);
    // (width, height, rowstride, has_alpha, bits_per_sample, channels, data)
    Some(Value::from((w, h, w * 4, true, 8, 4, thumb.into_raw())))
}

/// 【桌面通知】【缩略解码】将 png 逐行缩小，square 决定是否透明补齐正方形；返回非预乘 RGBA 或解码失败
fn thumbnail(png: &[u8], square: bool) -> Option<RgbaImage> {
    let preview =
        crate::utils::png_preview::load(Cursor::new(png), (THUMB, THUMB), &AtomicBool::new(false))
            .inspect_err(|error| info!("【桌面通知】【缩略解码】{error}"))
            .ok()?;
    let mut shot = RgbaImage::new(preview.width(), preview.height());
    for (target, source) in shot.pixels_mut().zip(preview.pixels()) {
        let color = source.demultiply();
        target.0 = [color.red(), color.green(), color.blue(), color.alpha()];
    }
    if !square {
        return Some(shot);
    }
    let (w, h) = shot.dimensions();
    let side = w.max(h);
    let mut square = RgbaImage::new(side, side);
    image::imageops::replace(
        &mut square,
        &shot,
        ((side - w) / 2).into(),
        ((side - h) / 2).into(),
    );
    Some(square)
}

#[cfg(test)]
#[path = "preview_tests.rs"]
mod preview_tests;

fn escape_markup(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn file_url(path: &Path) -> String {
    use std::fmt::Write;
    use std::os::unix::ffi::OsStrExt;

    let mut url = String::from("file://");
    for &b in path.as_os_str().as_bytes() {
        if b.is_ascii_alphanumeric() || b"/-._~".contains(&b) {
            url.push(b as char);
        } else {
            let _ = write!(url, "%{b:02X}");
        }
    }
    url
}
