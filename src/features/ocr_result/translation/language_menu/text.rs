//! 【OCR 翻译】【列表字体】使用原版弹层的 DejaVu Sans 十六像素字体，独立排版和省略不改变正文默认家族

use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping};

/// 【OCR 翻译】【列表排版】按 fonts、family 和 text 返回单行字形及宽度，不使用 OCR 正文的默认家族
fn shape(fonts: &mut FontSystem, family: &str, text: &str) -> (Buffer, f32) {
    let mut buffer = Buffer::new(fonts, Metrics::new(16.0, 19.0));
    buffer.set_size(None, None);
    buffer.set_wrap(cosmic_text::Wrap::None);
    buffer.set_text(
        text,
        &Attrs::new().family(Family::Name(family)),
        Shaping::Advanced,
        None,
    );
    buffer.shape_until_scroll(fonts, false);
    let width = buffer
        .layout_runs()
        .map(|run| run.line_w)
        .fold(0.0, f32::max)
        .round();
    (buffer, width)
}

/// 【OCR 翻译】【原版省略】按 fonts、family、text 和 width 返回带单个省略号的有界字形，保留 UTF-8 字符边界
fn elided(fonts: &mut FontSystem, family: &str, text: &str, width: f32) -> Buffer {
    let (full, measured) = shape(fonts, family, text);
    if measured <= width {
        return full;
    }
    let boundaries: Vec<_> = text
        .char_indices()
        .map(|(index, _)| index)
        .chain(std::iter::once(text.len()))
        .collect();
    let (mut left, mut right) = (0, boundaries.len() - 1);
    while left < right {
        let middle = (left + right).div_ceil(2);
        let candidate = format!("{}…", &text[..boundaries[middle]]);
        if shape(fonts, family, &candidate).1 <= width {
            left = middle;
        } else {
            right = middle - 1;
        }
    }
    shape(fonts, family, &format!("{}…", &text[..boundaries[left]])).0
}

/// 【OCR 翻译】【两态字形】按 fonts 和 popup_width 返回九项普通、选中两态缓冲，选中态为勾号留白，不在每次悬停时重排
pub(super) fn rows(
    fonts: &mut FontSystem,
    popup_width: f32,
    language: crate::ui::language::Language,
) -> Vec<[Buffer; 2]> {
    let family = crate::fonts::text_faces::family(fonts, "DejaVu Sans", false, false);
    crate::translation::language::labels(language)
        .iter()
        .map(|text| {
            [
                elided(fonts, &family, text, (popup_width - 44.0).max(1.0)),
                elided(fonts, &family, text, (popup_width - 62.0).max(1.0)),
            ]
        })
        .collect()
}
