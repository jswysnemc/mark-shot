pub(crate) mod fs_lock;
pub(crate) mod helper_process;
pub(crate) mod memory;
pub(crate) mod platform;
pub(crate) mod process_group;
pub(crate) mod screenshot_path;
#[cfg(test)]
pub(crate) mod test_files;
pub(crate) mod transparent_pixmap;

use crate::types::{CursorIcon, Placement, SelectionHandle, SignedRect};
use std::path::PathBuf;
use tiny_skia::Pixmap;
use tiny_skia::Rect;

#[inline]
pub fn make_rect(a: (f64, f64), b: (f64, f64)) -> Option<Rect> {
    let w = (a.0 - b.0).abs();
    let h = (a.1 - b.1).abs();
    if w < 1.0 || h < 1.0 {
        return None;
    }
    Rect::from_ltrb(
        a.0.min(b.0).floor() as f32,
        a.1.min(b.1).floor() as f32,
        a.0.max(b.0).ceil() as f32,
        a.1.max(b.1).ceil() as f32,
    )
}

pub fn global_point_to_local(
    placements: &[Placement],
    global: (f64, f64),
    fallback_idx: usize,
    fallback_local: (f64, f64),
) -> (usize, f64, f64) {
    let (gx, gy) = global;
    placements
        .iter()
        .enumerate()
        .find_map(|(idx, p)| {
            let (px, py) = p.position;
            let (w, h) = p.size;
            let inside =
                gx >= px as f64 && gx < (px + w) as f64 && gy >= py as f64 && gy < (py + h) as f64;
            inside.then_some((idx, gx - px as f64, gy - py as f64))
        })
        .unwrap_or((fallback_idx, fallback_local.0, fallback_local.1))
}

pub fn encode_png(pixmap: &Pixmap) -> Vec<u8> {
    encode_rgba(pixmap.data(), pixmap.width(), pixmap.height())
}

/// used for windows screenshots with some kind of transparency
/// like the same rounded corners.
/// Pixmap premultiplied, while png is not
///
/// alpha encoding is around 1.5 ms slower than direct encoding
pub fn encode_png_with_alpha(mut pixmap: Pixmap) -> Vec<u8> {
    let mut recip = [0u32; 256];
    for a in 1..256 {
        recip[a as usize] = ((255 << 16) + a / 2) / a;
    }
    for px in pixmap.data_mut().as_chunks_mut::<4>().0 {
        let a = px[3];
        if a != 255 {
            let k = recip[a as usize];
            for c in &mut px[..3] {
                *c = ((*c as u32 * k + (1 << 15)) >> 16).min(255) as u8;
            }
        }
    }
    encode_rgba(pixmap.data(), pixmap.width(), pixmap.height())
}

pub(crate) fn encode_rgba(rgba: &[u8], width: u32, height: u32) -> Vec<u8> {
    use image::ImageEncoder;
    use image::codecs::png::{CompressionType, FilterType, PngEncoder};

    let mut png_bytes = Vec::new();

    let encoder =
        PngEncoder::new_with_quality(&mut png_bytes, CompressionType::Fast, FilterType::Paeth);
    //  Why specifically Paeth ?

    //  filter type     | png encoded timing │ size
    //  --------------------------------------------
    //  Adaptive (prev) |       44.74        | 1.92 MB
    //  --------------------------------------------
    //  NoFilter        |       69.95        | 15.82 MB
    //  --------------------------------------------
    //  Sub             |       39.48       | 2.34 MB
    //  --------------------------------------------
    //  Up              |       38.19       | 2.19 MB
    //  --------------------------------------------
    //  Avg             |       39.93       | 2.68 MB
    //  --------------------------------------------
    //  Paeth           |       39.43       | 2.21 MB

    encoder
        .write_image(rgba, width, height, image::ExtendedColorType::Rgba8)
        .unwrap();

    png_bytes
}

pub fn save_to_file(
    png_data: &[u8],
    dir: Option<&std::path::Path>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = screenshot_path::planned(dir)?;
    let directory = path.parent().ok_or("invalid screenshot directory")?;
    let stem = path
        .file_stem()
        .and_then(|name| name.to_str())
        .ok_or("invalid screenshot filename")?;
    std::fs::create_dir_all(directory)?;
    Ok(write_unique(directory, stem, png_data, 0o666)?)
}

/// Writes `<stem>.png`, or `<stem>_N.png` if taken, never overwriting a file.
/// used for usual saving screenshots, and temp file for notif preview
pub fn write_unique(
    dir: &std::path::Path,
    stem: &str,
    data: &[u8],
    mode: u32,
) -> std::io::Result<PathBuf> {
    #[cfg(target_os = "linux")]
    use std::os::unix::fs::OpenOptionsExt;

    let mut n = 0;
    loop {
        let filename = match n {
            0 => format!("{stem}.png"),
            n => format!("{stem}_{n}.png"),
        };
        let path = dir.join(filename);
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(target_os = "linux")]
        options.mode(mode);
        #[cfg(target_os = "windows")]
        let _ = mode;
        let file = options.open(&path);
        match file {
            Ok(mut file) => {
                std::io::Write::write_all(&mut file, data)?;
                return Ok(path);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => n += 1,
            Err(e) => return Err(e),
        }
    }
}

pub fn write_to_stdout(data: &[u8], binary: bool) -> std::io::Result<()> {
    use std::io::{IsTerminal, Write};

    let mut stdout = std::io::stdout().lock();
    if binary && stdout.is_terminal() {
        return Err(std::io::Error::other(
            "stdout is a terminal, redirect it to a file or a pipe",
        ));
    }
    stdout.write_all(data)?;
    stdout.flush()
}

#[inline]
pub fn rects_overlap(a: &Rect, b: &Rect) -> bool {
    a.left() < b.right() && a.right() > b.left() && a.top() < b.bottom() && a.bottom() > b.top()
}

// to render necessary monitors
#[inline]
pub fn get_overlapping_monitors(selection: &Rect, placements: &[crate::types::Placement]) -> u32 {
    let mut mask = 0u32;
    for (i, p) in placements.iter().enumerate() {
        let mx = p.position.0 as f32;
        let my = p.position.1 as f32;
        let overlaps = selection.left() < mx + p.size.0 as f32
            && selection.right() > mx
            && selection.top() < my + p.size.1 as f32
            && selection.bottom() > my;
        if overlaps {
            mask |= 1 << i;
        }
    }
    mask
}

// used for selecting/resizing annotations or selection
#[inline]
pub fn hit_test_rect_handle(sel: &Rect, pos: (f64, f64)) -> SelectionHandle {
    let (x, y) = pos;
    let (l, r, t, b) = (
        sel.left() as f64,
        sel.right() as f64,
        sel.top() as f64,
        sel.bottom() as f64,
    );
    let w = sel.width() as f64;
    let h = sel.height() as f64;

    let input = &crate::config::get().input;
    let (ratio, min, max) = (
        input.corner_ratio as f64,
        input.corner_min as f64,
        input.corner_max as f64,
    );
    let corner_w = (w * ratio).clamp(min, max).min(w * 0.5);
    let corner_h = (h * ratio).clamp(min, max).min(h * 0.5);

    let half_pad = input.handle_hit_width as f64 / 2.0;

    // Top-Left
    let in_tl_horizontal = (y - t).abs() <= half_pad && x >= l - half_pad && x <= l + corner_w;
    let in_tl_vertical = (x - l).abs() <= half_pad && y >= t - half_pad && y <= t + corner_h;
    if in_tl_horizontal || in_tl_vertical {
        return SelectionHandle::TopLeft;
    }

    // Top-Right
    let in_tr_horizontal = (y - t).abs() <= half_pad && x >= r - corner_w && x <= r + half_pad;
    let in_tr_vertical = (x - r).abs() <= half_pad && y >= t - half_pad && y <= t + corner_h;
    if in_tr_horizontal || in_tr_vertical {
        return SelectionHandle::TopRight;
    }

    // Bottom-Left
    let in_bl_horizontal = (y - b).abs() <= half_pad && x >= l - half_pad && x <= l + corner_w;
    let in_bl_vertical = (x - l).abs() <= half_pad && y >= b - corner_h && y <= b + half_pad;
    if in_bl_horizontal || in_bl_vertical {
        return SelectionHandle::BottomLeft;
    }

    // Bottom-Right
    let in_br_horizontal = (y - b).abs() <= half_pad && x >= r - corner_w && x <= r + half_pad;
    let in_br_vertical = (x - r).abs() <= half_pad && y >= b - corner_h && y <= b + half_pad;
    if in_br_horizontal || in_br_vertical {
        return SelectionHandle::BottomRight;
    }

    // Top
    if (y - t).abs() <= half_pad && x > l + corner_w && x < r - corner_w {
        return SelectionHandle::Top;
    }
    // Bottom
    if (y - b).abs() <= half_pad && x > l + corner_w && x < r - corner_w {
        return SelectionHandle::Bottom;
    }
    // Left
    if (x - l).abs() <= half_pad && y > t + corner_h && y < b - corner_h {
        return SelectionHandle::Left;
    }
    // Right
    if (x - r).abs() <= half_pad && y > t + corner_h && y < b - corner_h {
        return SelectionHandle::Right;
    }

    if x >= l + half_pad && x <= r - half_pad && y >= t + half_pad && y <= b - half_pad {
        return SelectionHandle::Move;
    }

    SelectionHandle::None
}

/// 【界面】【控制点光标】根据 handle 与 dragging 返回移动、缩放或定点光标；空控制点返回空值
pub fn cursor_for_handle(handle: SelectionHandle, dragging: bool) -> Option<CursorIcon> {
    Some(match handle.generic_rect_handle() {
        SelectionHandle::TopLeft | SelectionHandle::BottomRight => CursorIcon::NwseResize,
        SelectionHandle::TopRight | SelectionHandle::BottomLeft => CursorIcon::NeswResize,
        SelectionHandle::Top | SelectionHandle::Bottom => CursorIcon::NsResize,
        SelectionHandle::Left | SelectionHandle::Right => CursorIcon::EwResize,
        SelectionHandle::Move | SelectionHandle::Rotate | SelectionHandle::NumberBubble => {
            if dragging {
                CursorIcon::Grabbing
            } else {
                CursorIcon::Grab
            }
        }
        SelectionHandle::AngleVertex
        | SelectionHandle::AngleFirstArm
        | SelectionHandle::AngleSecondArm
        | SelectionHandle::LinePoint(_)
        | SelectionHandle::NumberTip => CursorIcon::Crosshair,
        _ => return None,
    })
}

#[inline]
/// 【界面】【矩形控制点】根据 orig、handle 与 delta 计算有符号范围；尺子专用定点和旋转不改变矩形
pub fn apply_handle_drag(orig: &Rect, handle: SelectionHandle, delta: (f64, f64)) -> SignedRect {
    let (dx, dy) = (delta.0 as f32, delta.1 as f32);
    let (mut l, mut r, mut t, mut b) = (orig.left(), orig.right(), orig.top(), orig.bottom());
    match handle.generic_rect_handle() {
        SelectionHandle::TopLeft => {
            l += dx;
            t += dy;
        }
        SelectionHandle::Top => {
            t += dy;
        }
        SelectionHandle::TopRight => {
            r += dx;
            t += dy;
        }
        SelectionHandle::Left => {
            l += dx;
        }
        SelectionHandle::Right => {
            r += dx;
        }
        SelectionHandle::BottomLeft => {
            l += dx;
            b += dy;
        }
        SelectionHandle::Bottom => {
            b += dy;
        }
        SelectionHandle::BottomRight => {
            r += dx;
            b += dy;
        }
        SelectionHandle::Move => {
            l += dx;
            r += dx;
            t += dy;
            b += dy;
        }
        SelectionHandle::None
        | SelectionHandle::AngleVertex
        | SelectionHandle::AngleFirstArm
        | SelectionHandle::AngleSecondArm
        | SelectionHandle::Rotate => {}
        _ => {}
    }
    SignedRect {
        left: l,
        top: t,
        right: r,
        bottom: b,
    }
}

pub fn get_full_workspace_rect(placements: &[Placement]) -> Option<Rect> {
    if placements.is_empty() {
        return None;
    }

    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;

    for p in placements {
        min_x = min_x.min(p.position.0);
        min_y = min_y.min(p.position.1);
        max_x = max_x.max(p.position.0 + p.size.0);
        max_y = max_y.max(p.position.1 + p.size.1);
    }

    Rect::from_ltrb(min_x as f32, min_y as f32, max_x as f32, max_y as f32)
}

/// pixels swap
/// drops or keeps alhpa, for now only kde may
/// send a proper screenshot of a transparent window
#[inline]
pub fn to_rgba(pixels: &mut [u8], bgr: bool, keep_alpha: bool) {
    for chunk in pixels.as_chunks_mut::<4>().0 {
        if bgr {
            chunk.swap(0, 2);
        }
        if !keep_alpha {
            chunk[3] = 255;
        }
    }
}

pub fn premultiply(rgba: &mut [u8]) {
    for px in rgba.as_chunks_mut::<4>().0 {
        let a = px[3] as u16;
        if a != 255 {
            for c in &mut px[..3] {
                *c = ((*c as u16 * a + 127) / 255) as u8;
            }
        }
    }
}

/// Copies into a shm canvas with R/B already swapped, in a single pass.
///
/// Copying first and swizzling the canvas afterwards leaves the buffer holding
/// un-swapped bytes for the length of the second pass, and there is no
/// wl_buffer.release tracking here - so a compositor sampling in that window
/// shows a frame with red and blue exchanged. One pass per pixel means the
/// buffer only ever holds correct colors.
pub fn copy_swizzled(dst: &mut [u8], src: &[u8]) {
    for (d, s) in dst
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip(src.as_chunks::<4>().0)
    {
        d[0] = s[2];
        d[1] = s[1];
        d[2] = s[0];
        d[3] = s[3];
    }
}

/// 【辅助进程】【独立调用】将 args 和 stdin 交给独立应用进程；返回启动及输入结果，终端中断不影响辅助窗口
pub fn spawn_self(args: &[&str], stdin: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    spawn_self_with(args, stdin, std::process::Stdio::null()).map(|_| ())
}

/// 【辅助进程】【就绪交接】将 args、stdin 交给独立进程，在 timeout 内等待首行；返回就绪结果或错误
pub fn spawn_self_ready(
    args: &[&str],
    stdin: &[u8],
    timeout: std::time::Duration,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::BufRead;

    let stdout =
        spawn_self_with(args, stdin, std::process::Stdio::piped())?.ok_or("child has no stdout")?;

    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        let read = std::io::BufReader::new(stdout).read_line(&mut line);
        let _ = tx.send(read.map(|_| line));
    });

    match rx.recv_timeout(timeout) {
        Ok(Ok(line)) if line == "\n" => Ok(()),
        Ok(Ok(line)) if line.is_empty() => Err("helper process exited unexpectedly".into()),
        Ok(Ok(line)) => Err(line.trim_end().into()),
        Ok(Err(e)) => Err(e.into()),
        Err(_) => Err("helper process didn't respond in time".into()),
    }
}

/// 【辅助进程】【管道调用】启动 args 对应进程并写入 stdin；stdout 指定输出方式，返回可选输出管道
fn spawn_self_with(
    args: &[&str],
    stdin: &[u8],
    stdout: std::process::Stdio,
) -> Result<Option<std::process::ChildStdout>, Box<dyn std::error::Error>> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut command = Command::new(std::env::current_exe()?);
    command.args(args);
    crate::config::pass_to(&mut command);
    command
        .stdin(Stdio::piped())
        .stdout(stdout)
        // 1. 【辅助进程】【日志隔离】错误输出使用日志文件，避免长驻进程保持调用方输出管道
        .stderr(crate::logging::open_log_file().map_or_else(Stdio::null, Stdio::from));
    let mut pipes = helper_process::spawn(command)?;

    pipes
        .stdin
        .take()
        .ok_or("child has no stdin")?
        .write_all(stdin)?;
    Ok(pipes.stdout.take())
}

#[cfg(target_os = "linux")]
pub fn copy_to_clipboard(text: &str) -> Result<(), Box<dyn std::error::Error>> {
    use crate::backend::clipboard::{DAEMON_ARG, READY_TIMEOUT, TEXT_ARG};

    spawn_self_ready(&[DAEMON_ARG, TEXT_ARG], text.as_bytes(), READY_TIMEOUT)
}

#[cfg(target_os = "linux")]
pub fn copy_to_primary(text: &str) -> Result<(), Box<dyn std::error::Error>> {
    use crate::backend::clipboard::{DAEMON_ARG, PRIMARY_ARG, READY_TIMEOUT, TEXT_ARG};

    spawn_self_ready(
        &[DAEMON_ARG, TEXT_ARG, PRIMARY_ARG],
        text.as_bytes(),
        READY_TIMEOUT,
    )
}

#[cfg(target_os = "linux")]
pub fn paste_from_clipboard() -> Option<String> {
    use std::io::Read;
    use wl_clipboard_rs::paste::{ClipboardType, MimeType, Seat, get_contents};

    let (mut pipe, _) =
        get_contents(ClipboardType::Regular, Seat::Unspecified, MimeType::Text).ok()?;
    let mut text = String::new();
    pipe.read_to_string(&mut text).ok()?;
    Some(text)
}

#[cfg(target_os = "windows")]
/// 【Windows剪贴板】【文字输出】按text直接交付系统Unicode剪贴板，返回结果，不启动后台进程
pub fn copy_to_clipboard(text: &str) -> crate::AppResult {
    crate::backend::clipboard::copy_text(text)
}
#[cfg(target_os = "windows")]
/// 【Windows剪贴板】【主选择兼容】按text交付系统剪贴板，返回结果，Windows没有独立主选择区
pub fn copy_to_primary(text: &str) -> crate::AppResult {
    copy_to_clipboard(text)
}
#[cfg(target_os = "windows")]
/// 【Windows剪贴板】【文字粘贴】无参数，返回系统Unicode正文或不可用状态
pub fn paste_from_clipboard() -> Option<String> {
    crate::backend::clipboard::read_text().ok()
}

#[cfg(test)]
mod tests {
    use super::{encode_png_with_alpha, write_unique};

    #[test]
    fn saving_twice_with_the_same_name_keeps_both_files() {
        let dir = std::env::temp_dir().join(format!("lumine-save-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let first = write_unique(&dir, "shot", b"one", 0o666).unwrap();
        let second = write_unique(&dir, "shot", b"two", 0o666).unwrap();

        assert_eq!(first.file_name().unwrap(), "shot.png");
        assert_eq!(second.file_name().unwrap(), "shot_1.png");
        assert_eq!(std::fs::read(&first).unwrap(), b"one");
        assert_eq!(std::fs::read(&second).unwrap(), b"two");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn written_file_gets_the_given_mode() {
        use std::os::unix::fs::PermissionsExt;

        let dir = std::env::temp_dir().join(format!("lumine-mode-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let path = write_unique(&dir, "shot", b"png", 0o400).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();

        assert_eq!(mode & 0o777, 0o400);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn png_with_alpha_stores_straight_colors() {
        let size = tiny_skia::IntSize::from_wh(3, 1).unwrap();
        let data = vec![0, 0, 0, 0, 64, 32, 16, 128, 10, 20, 30, 255];
        let pixmap = tiny_skia::Pixmap::from_vec(data, size).unwrap();

        let png = encode_png_with_alpha(pixmap);
        let decoded = image::load_from_memory(&png).unwrap().into_rgba8();

        assert_eq!(
            decoded.into_raw(),
            [0, 0, 0, 0, 128, 64, 32, 128, 10, 20, 30, 255]
        );
    }
}
