// ── KDE KWin ScreenShot2 DBus capture backend ──────────────────────────────
//
// Bypasses xdg-desktop-portal entirely, uses kwin's private protocol which is faster
// and doesn't ask for monitor choice as Portal does

use log::info;
use std::collections::HashMap;
use std::error::Error;
use std::os::fd::{AsFd, OwnedFd};

use async_trait::async_trait;
use tokio::sync::OnceCell;
use zbus::zvariant::{Fd, OwnedValue, Value};
use zbus::{Connection, proxy};

use crate::backend::CaptureMethod;
use crate::types::{Capture, CaptureResult, MonitorFrame, Output, StreamInfo};
use crate::utils::{premultiply, to_rgba};

// async_trait requires the whole future graph to be Send; std::error::Error
// alone isn't Send, so all internal helpers use this bound instead and only
// convert to the trait's plain Box<dyn Error> at the outer return boundary
type BoxErr = Box<dyn Error + Send + Sync>;

#[proxy(
    interface = "org.kde.KWin.ScreenShot2",
    default_service = "org.kde.KWin",
    default_path = "/org/kde/KWin/ScreenShot2"
)]
trait ScreenShot2 {
    async fn capture_screen(
        &self,
        name: &str,
        options: HashMap<&str, Value<'_>>,
        pipe: Fd<'_>,
    ) -> zbus::Result<HashMap<String, OwnedValue>>;

    async fn capture_active_window(
        &self,
        options: HashMap<&str, Value<'_>>,
        pipe: Fd<'_>,
    ) -> zbus::Result<HashMap<String, OwnedValue>>;
}

#[derive(Clone, Copy)]
enum Target<'a> {
    Screen(&'a str),
    ActiveWindow,
}

impl Target<'_> {
    fn label(self) -> String {
        match self {
            Target::Screen(name) => format!("output '{name}'"),
            Target::ActiveWindow => "the active window".into(),
        }
    }
}

struct Shot {
    pixels: Vec<u8>,
    width: u32,
    height: u32,
    scale: Option<f64>,
}

const READ_CHUNK: u64 = 256 * 1024;

pub struct KdeMethod {
    conn: OnceCell<Connection>,
    include_cursor: bool,
}

impl Default for KdeMethod {
    fn default() -> Self {
        Self::new()
    }
}

impl KdeMethod {
    pub fn new() -> Self {
        Self::with_cursor(false)
    }

    /// 【KWin】【捕获初始化】创建按需连接的捕获后端；include_cursor 控制指针合成
    pub fn with_cursor(include_cursor: bool) -> Self {
        Self {
            conn: OnceCell::new(),
            include_cursor,
        }
    }

    async fn session_bus(&self) -> Result<&Connection, BoxErr> {
        self.conn
            .get_or_try_init(|| async { Connection::session().await.map_err(BoxErr::from) })
            .await
    }
}

// without this its going to break on new kde version
fn is_bgr(format: Option<u32>) -> Option<bool> {
    match format {
        None | Some(4..=6) => Some(true),
        Some(16..=18) => Some(false),
        Some(_) => None,
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Alpha {
    Opaque,
    Straight,
    Premultiplied,
}

// QImage::Format: 4 RGB32, 5 ARGB32, 6 ARGB32_Premultiplied,
// 16 RGBX8888, 17 RGBA8888, 18 RGBA8888_Premultiplied
fn alpha_of(format: Option<u32>) -> Alpha {
    match format {
        Some(5 | 17) => Alpha::Straight,
        Some(6 | 18) => Alpha::Premultiplied,
        _ => Alpha::Opaque,
    }
}

// captures one output or the active window; returns tight (unpadded) pixels
async fn capture_one(
    proxy: &ScreenShot2Proxy<'_>,
    target: Target<'_>,
    dimensions: Option<(usize, usize)>,
    include_cursor: bool,
) -> Result<Shot, BoxErr> {
    let label = target.label();
    let (read_fd, write_fd): (OwnedFd, OwnedFd) = nix::unistd::pipe()?;
    let (layout_tx, layout_rx) = std::sync::mpsc::channel::<(bool, bool)>();

    let read_task = tokio::task::spawn_blocking(move || -> std::io::Result<Vec<u8>> {
        use std::io::Read;
        let mut buf = match dimensions {
            Some((w, h)) => Vec::with_capacity(w.saturating_mul(h).saturating_mul(4)),
            None => Vec::new(),
        };
        let file = std::fs::File::from(read_fd);
        // initially swizzle was starting only Kwin fully gives us the frame
        // now we start swizzling immediately even before kwin finishing, just swizzling what we already have
        // for performance gain
        let mut order = None;
        let mut converted = 0;
        while (&file).take(READ_CHUNK).read_to_end(&mut buf)? > 0 {
            let (bgr, keep_alpha) = match order {
                Some(layout) => layout,
                None => *order.insert(layout_rx.recv().map_err(|_| {
                    std::io::Error::other("KWin didn't report the screenshot format")
                })?),
            };
            let ready = buf.len() - buf.len() % 4;
            to_rgba(&mut buf[converted..ready], bgr, keep_alpha);
            converted = ready;
        }
        Ok(buf)
    });

    // apparently without this option, scaling picture comes with reducing screenshots quality
    let mut options = HashMap::from([("native-resolution", Value::from(true))]);
    options.insert("include-cursor", Value::from(include_cursor));
    options.insert(
        "hide-caller-windows",
        Value::from(
            crate::config::preferences::current()
                .capture
                .hide_own_windows,
        ),
    );
    let result = match target {
        Target::Screen(name) => {
            proxy
                .capture_screen(name, options, Fd::from(write_fd.as_fd()))
                .await
        }
        Target::ActiveWindow => {
            // honestly copied from spectacle, with decorations and without shadow
            // you can disable it in spectacle launch options, but i find it kinda useless
            options.insert("include-decoration", Value::from(true));
            options.insert("include-shadow", Value::from(false));
            proxy
                .capture_active_window(options, Fd::from(write_fd.as_fd()))
                .await
        }
    };

    drop(write_fd);

    let metadata = result?;
    let format = metadata
        .get("format")
        .and_then(|v| u32::try_from(v.clone()).ok());
    let bgr = is_bgr(format).ok_or(format!(
        "KWin sent the screenshot of {label} in QImage format {format:?}, which isn't supported"
    ))?;
    // probably useless, but a small safery check won't hurt
    let alpha = match target {
        Target::Screen(_) => Alpha::Opaque,
        Target::ActiveWindow => alpha_of(format),
    };
    let _ = layout_tx.send((bgr, alpha != Alpha::Opaque));
    let mut raw = read_task.await??;
    // pixmap worrks with premultiplied colors, kwin 6.7 already sends them,
    // so this is just in case
    if alpha == Alpha::Straight {
        premultiply(&mut raw);
    }

    let width = metadata
        .get("width")
        .and_then(|v| u32::try_from(v.clone()).ok())
        .ok_or(format!("no 'width' for {label}"))?;
    let height = metadata
        .get("height")
        .and_then(|v| u32::try_from(v.clone()).ok())
        .ok_or(format!("no 'height' for {label}"))?;
    let stride = metadata
        .get("stride")
        .and_then(|v| u32::try_from(v.clone()).ok())
        .unwrap_or(width * 4);
    let scale = metadata
        .get("scale")
        .and_then(|v| f64::try_from(v.clone()).ok());

    let row_bytes = (width * 4) as usize;
    let needed = stride as usize * height as usize;

    // I have no idea how to replicate "missing pixels bug" btw
    // besides World of Tanks, only there screenshots end up without
    // 1024 in the last sequence, initially I thought its my fault
    // but spectacle had the same result
    if raw.len() < needed {
        let missing = needed - raw.len();
        info!("warning: short read for {label}: padding {missing} missing bytes with transparent");
        raw.resize(needed, 0); // zero-pad remaining tail with transparent pixels
    }

    // if there is no padding, there is no point in allocations and copying
    if stride as usize == row_bytes {
        raw.truncate(row_bytes * height as usize);
        return Ok(Shot {
            pixels: raw,
            width,
            height,
            scale,
        });
    }

    // unless there is, we need to do heavy copy
    let mut tight = vec![0u8; row_bytes * height as usize];
    for row in 0..height as usize {
        let src = &raw[row * stride as usize..][..row_bytes];
        let dst = &mut tight[row * row_bytes..][..row_bytes];
        dst.copy_from_slice(src);
    }

    Ok(Shot {
        pixels: tight,
        width,
        height,
        scale,
    })
}

#[async_trait]
impl CaptureMethod for KdeMethod {
    /// 【持续捕获】【KWin会话】按 outputs 创建复用系统总线的捕获后端，返回固定目标会话
    async fn open_session(
        &self,
        outputs: &[Output],
    ) -> crate::backend::capture_session::Result<
        Box<dyn crate::backend::capture_session::CaptureSession>,
    > {
        let method = Self::with_cursor(self.include_cursor);
        method.session_bus().await?;
        Ok(crate::backend::capture_session::repeated(
            Box::new(method),
            outputs,
        ))
    }
    async fn capture_frame(&self, outputs: &[Output]) -> Result<CaptureResult, Box<dyn Error>> {
        let inner = async {
            let proxy = ScreenShot2Proxy::new(self.session_bus().await?).await?;

            // one CaptureScreen call per monitor, all concurrent
            let futs = outputs.iter().map(|o| {
                let proxy = proxy.clone();
                let name = o.info.name.clone().unwrap_or_default();

                let dimensions = o
                    .info
                    .modes
                    .iter()
                    .find(|m| m.current)
                    .or_else(|| o.info.modes.first())
                    .map(|m| (m.dimensions.0 as usize, m.dimensions.1 as usize));

                async move {
                    capture_one(
                        &proxy,
                        Target::Screen(&name),
                        dimensions,
                        self.include_cursor,
                    )
                    .await
                }
            });
            futures::future::try_join_all(futs).await
        };

        let results: Vec<_> = inner.await.map_err(|e: BoxErr| -> Box<dyn Error> { e })?;

        // try_join_all preserves input order, so results[i] <-> outputs[i] —
        // no reconciliation step needed, unlike the portal backend
        let frames = results
            .into_iter()
            .zip(outputs)
            .enumerate()
            .map(|(output, (shot, o))| MonitorFrame {
                output,
                pixels: shot.pixels,
                pw_width: shot.width,
                pw_height: shot.height,
                pw_stride: shot.width * 4,
                info: StreamInfo {
                    node_id: 0,
                    size: o.info.logical_size,
                    position: o.info.logical_position,
                },
            })
            .collect();

        Ok(CaptureResult { frames })
    }

    async fn capture_active_window(&self) -> Result<Capture, Box<dyn Error>> {
        let inner = async {
            let proxy = ScreenShot2Proxy::new(self.session_bus().await?).await?;
            capture_one(&proxy, Target::ActiveWindow, None, self.include_cursor).await
        };
        let shot = inner.await.map_err(|e: BoxErr| -> Box<dyn Error> { e })?;
        let size = tiny_skia::IntSize::from_wh(shot.width, shot.height)
            .ok_or_else(|| format!("KWin sent an empty window: {}x{}", shot.width, shot.height))?;
        let pixmap = tiny_skia::Pixmap::from_vec(shot.pixels, size)
            .ok_or("KWin sent fewer pixels than the window size")?;
        Ok(Capture {
            pixmap: pixmap.into(),
            scale: shot.scale.unwrap_or(1.0) as f32,
        })
    }

    /// Used only for delayed screenshots
    /// since connecting to zbus take some time, and can be completed ahead
    /// of the timing for getting screenshot
    async fn prepare(&self) -> Result<(), Box<dyn Error>> {
        self.session_bus()
            .await
            .map_err(|e| -> Box<dyn Error> { e })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argb32_formats_are_swapped() {
        for format in [4, 5, 6] {
            assert_eq!(is_bgr(Some(format)), Some(true));
        }
    }

    #[test]
    fn rgba8888_formats_keep_their_order() {
        for format in [16, 17, 18] {
            assert_eq!(is_bgr(Some(format)), Some(false));
        }
    }

    #[test]
    fn missing_format_means_argb32() {
        assert_eq!(is_bgr(None), Some(true));
    }

    #[test]
    fn other_formats_are_rejected() {
        for format in [0, 13, 22, 26, 30] {
            assert_eq!(is_bgr(Some(format)), None);
        }
    }

    #[test]
    /// yo is this https://files.catbox.moe/kx68bc
    fn kwin_6_7_bytes_become_rgba() {
        let mut px = vec![0x30, 0x20, 0x10, 0xFF];
        to_rgba(&mut px, is_bgr(Some(6)).unwrap(), false);
        assert_eq!(px, [0x10, 0x20, 0x30, 0xFF]);
    }

    #[test]
    fn rgbx_bytes_become_opaque_rgba() {
        let mut px = vec![0x10, 0x20, 0x30, 0x00];
        to_rgba(&mut px, is_bgr(Some(16)).unwrap(), false);
        assert_eq!(px, [0x10, 0x20, 0x30, 0xFF]);
    }
}
