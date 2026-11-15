//! 【贴图】【内容绘制】文本和色卡使用同一主题与系统字体

use super::IMAGE_LIMIT;
use crate::{
    AppResult,
    renderer::{Painter, text::TextStyle},
};
use cosmic_text::SwashCache;
use tiny_skia::{Pixmap, Rect};

/// 【贴图】【色卡绘制】按 RGBA 绘制颜色样本和 HEX、RGB 数值，返回紧凑 PNG
pub(super) fn color_image(color: [u8; 4]) -> AppResult<Vec<u8>> {
    let [red, green, blue, alpha] = color;
    let label = format!(
        "#{red:02X}{green:02X}{blue:02X}{}\nRGB: {red}, {green}, {blue}",
        if alpha == 255 {
            String::new()
        } else {
            format!("{alpha:02X}")
        }
    );
    let text = Pixmap::decode_png(&text_image(&label)?)?;
    let mut image =
        Pixmap::new(text.width(), text.height() + 76).ok_or("cannot allocate color card")?;
    image.fill(crate::config::get().theme.background.color());
    image.draw_pixmap(
        0,
        76,
        text.as_ref(),
        &tiny_skia::PixmapPaint::default(),
        tiny_skia::Transform::identity(),
        None,
    );
    let bounds = Rect::from_xywh(16.0, 16.0, image.width() as f32 - 32.0, 60.0)
        .ok_or("invalid color card geometry")?;
    Painter::new(&mut image).fill_rounded(
        bounds,
        6.0,
        tiny_skia::Color::from_rgba8(red, green, blue, alpha),
    );
    Ok(crate::utils::encode_png_with_alpha(image))
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
pub(super) fn text_image(text: &str) -> AppResult<Vec<u8>> {
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
