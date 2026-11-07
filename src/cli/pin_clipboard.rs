#[cfg(target_os = "linux")]
use std::io::Read;

use cosmic_text::SwashCache;
use tiny_skia::{Pixmap, Rect};
#[cfg(target_os = "linux")]
use wl_clipboard_rs::paste::{ClipboardType, MimeType, Seat, get_contents};

use crate::AppResult;
use crate::renderer::{Painter, text::TextStyle};

const IMAGE_LIMIT: u64 = 128 * 1024 * 1024;
#[cfg(target_os = "linux")]
const TEXT_LIMIT: u64 = 64 * 1024;

/// 【贴图】【剪贴板读取】读取指定 MIME 数据；超过上限时返回错误，不无界增长缓冲
#[cfg(target_os = "linux")]
fn read(mime: MimeType<'_>, limit: u64) -> AppResult<Vec<u8>> {
    let (reader, _) = get_contents(ClipboardType::Regular, Seat::Unspecified, mime)?;
    let mut bytes = Vec::new();
    reader.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err("clipboard content exceeds the pin size limit".into());
    }
    Ok(bytes)
}

/// 【贴图】【文本换行】按 Unicode 字符保留换行并拆分长行；返回用于排版的完整文本行
fn lines(text: &str) -> Vec<String> {
    text.split('\n')
        .flat_map(|line| {
            let characters: Vec<_> = line.chars().collect();
            if characters.is_empty() {
                return vec![String::new()];
            }
            characters
                .chunks(60)
                .map(|chunk| chunk.iter().collect())
                .collect()
        })
        .collect()
}

/// 【贴图】【文字图像】使用系统字体绘制完整剪贴板文本；返回可独立显示的 PNG
fn text_image(text: &str) -> AppResult<Vec<u8>> {
    if text.trim().is_empty() {
        return Err("the clipboard contains no image or text".into());
    }
    let rows = lines(text);
    let mut fonts = crate::fonts::bundled();
    crate::fonts::add_system(&mut fonts, crate::fonts::system());
    let mut cache = SwashCache::new();
    let mut probe = Pixmap::new(1, 1).ok_or("cannot allocate text probe")?;
    let mut painter = Painter::new(&mut probe).with_text(&mut fonts, &mut cache);
    let width = rows
        .iter()
        .map(|line| painter.measure(line, 16.0))
        .fold(120.0_f32, f32::max)
        .ceil() as u32
        + 32;
    let height = (rows.len() as u32)
        .checked_mul(24)
        .and_then(|height| height.checked_add(32))
        .ok_or("clipboard text is too large")?;
    if u64::from(width) * u64::from(height) * 4 > IMAGE_LIMIT {
        return Err("clipboard text image exceeds the pin size limit".into());
    }
    let mut frame = Pixmap::new(width, height).ok_or("cannot allocate clipboard text image")?;
    frame.fill(crate::config::get().theme.background.color());
    let mut painter = Painter::new(&mut frame).with_text(&mut fonts, &mut cache);
    for (index, line) in rows.iter().enumerate() {
        let rect = Rect::from_xywh(16.0, 16.0 + index as f32 * 24.0, width as f32 - 32.0, 24.0)
            .ok_or("invalid text geometry")?;
        painter.text(
            rect,
            line,
            TextStyle::new(16.0, crate::theme::color::foreground().color()),
        );
    }
    Ok(crate::utils::encode_png_with_alpha(frame))
}

/// 【贴图】【剪贴板入口】优先贴图 PNG 图片，否则绘制纯文字；不依赖外部程序
#[cfg(target_os = "linux")]
pub(super) fn run() -> AppResult {
    let image = match read(MimeType::Specific("image/png"), IMAGE_LIMIT) {
        Ok(image) => image,
        Err(_) => text_image(&String::from_utf8(read(MimeType::Text, TEXT_LIMIT)?)?)?,
    };
    crate::backend::pin::run(&image, 1.0)
}

#[cfg(target_os = "windows")]
/// 【Windows贴图】【剪贴板入口】无参数，优先读取原生PNG或DIB，否则绘制Unicode正文，返回独立贴图结果
pub(super) fn run() -> AppResult {
    let image = crate::backend::clipboard::read_image_png()
        .or_else(|_| crate::backend::clipboard::read_text().and_then(|text| text_image(&text)))?;
    crate::backend::pin::run(&image, 1.0)
}
