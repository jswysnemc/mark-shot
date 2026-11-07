//! 【图片翻译】【定向字形】使用原版 Noto Sans 家族，保留字形实际宽度与字体基线供排版和绘制共用

use cosmic_text::fontdb::{Family, Query, Stretch, Style};
use cosmic_text::{Attrs, Buffer, FontSystem, LayoutLine, Metrics, Shaping, Weight, Wrap};

pub(super) struct Text {
    pub size: f32,
    pub height: f64,
    pub ascent: f64,
    pub advance: f64,
    pub lines: Vec<String>,
}

/// 【图片翻译】【字体准备】在 fonts 中定向加载原版字体及 text 实际缺字，无返回值，不扫描整个系统字库
pub(crate) fn prepare(fonts: &mut FontSystem, text: &str) {
    crate::fonts::text_faces::family(fonts, "Noto Sans", false, false);
    crate::fonts::ocr_ui::ensure_text(fonts, text);
}

/// 【图片翻译】【字体度量】按 fonts、size 返回基线以上高度与总行高，使用同一字体的设计单位并量化到原版六十四分之一像素
fn metrics(fonts: &mut FontSystem, size: f32) -> (f64, f64) {
    let id = fonts.db().query(&Query {
        families: &[Family::Name("Noto Sans")],
        weight: Weight::NORMAL,
        stretch: Stretch::Normal,
        style: Style::Normal,
    });
    let Some(font) = id.and_then(|id| fonts.get_font(id, Weight::NORMAL)) else {
        return (f64::from(size), f64::from(size) * 1.3);
    };
    let metrics = font.metrics();
    let scale = f64::from(size) / f64::from(metrics.units_per_em.max(1));
    let ascent = (f64::from(metrics.ascent) * scale * 64.0).floor() / 64.0;
    let descent = (f64::from(metrics.descent).abs() * scale * 64.0).floor() / 64.0;
    (ascent, ascent + descent)
}

/// 【图片翻译】【正文缓冲】按 fonts、text、size、height 和 width 返回同一字体的字形缓冲，允许西文词间或任意字符换行
pub(super) fn buffer(
    fonts: &mut FontSystem,
    text: &str,
    size: f32,
    height: f32,
    width: Option<f32>,
) -> Buffer {
    let mut buffer = Buffer::new(fonts, Metrics::new(size, height));
    buffer.set_size(width, None);
    buffer.set_wrap(Wrap::WordOrGlyph);
    buffer.set_text(
        text,
        &Attrs::new().family(Family::Name("Noto Sans")),
        Shaping::Advanced,
        None,
    );
    buffer.shape_until_scroll(fonts, false);
    buffer
}

/// 【图片翻译】【定点布局】按 fonts、text、size、height 和 width 返回定点步进布局，在换行前量化字宽，保留双向文字簇
pub(super) fn glyph_lines(
    fonts: &mut FontSystem,
    text: &str,
    size: f32,
    height: f32,
    width: Option<f32>,
) -> Vec<LayoutLine> {
    let buffer = buffer(fonts, text, size, height, None);
    crate::fonts::fixed_layout::lines(fonts, &buffer, size, width, Wrap::WordOrGlyph)
}

/// 【图片翻译】【限定排版】按 fonts、text、size、width、source_pitch 与 start_size 返回字形行及原版比例行距
pub(super) fn shape(
    fonts: &mut FontSystem,
    text: &str,
    size: f32,
    width: f64,
    source_pitch: f64,
    start_size: f64,
) -> Text {
    let (ascent, height) = metrics(fonts, size);
    let mut lines = Vec::new();
    for paragraph in text.lines() {
        for run in glyph_lines(
            fonts,
            paragraph,
            size,
            height as f32,
            Some(width.max(1.0) as f32),
        ) {
            let Some(start) = run.glyphs.iter().map(|glyph| glyph.start).min() else {
                continue;
            };
            let Some(end) = run.glyphs.iter().map(|glyph| glyph.end).max() else {
                continue;
            };
            let Some(text) = paragraph.get(start..end).map(str::trim) else {
                continue;
            };
            if !text.is_empty() {
                lines.push(text.to_owned());
            }
        }
    }
    Text {
        size,
        height,
        ascent,
        advance: (source_pitch * f64::from(size) / start_size).clamp(height * 1.12, height * 1.8),
        lines,
    }
}

/// 【图片翻译】【字形宽度】按 fonts、text、size 和 height 返回未换行的实际字形宽度
pub(crate) fn width(fonts: &mut FontSystem, text: &str, size: f32, height: f64) -> f64 {
    glyph_lines(fonts, text, size, height as f32, None)
        .iter()
        .map(|run| f64::from(run.w))
        .fold(0.0, f64::max)
}

/// 【图片翻译】【尾行截断】按 fonts、text、size、height 和 width 返回原版右侧省略文本，只调整显示行，不改完整译文
pub(super) fn elide(
    fonts: &mut FontSystem,
    text: &str,
    size: f32,
    height: f64,
    width: f64,
) -> String {
    let original = format!("{text}…");
    if self::width(fonts, &original, size, height) <= width {
        return original;
    }
    let mut result = text.to_owned();
    while !result.is_empty() {
        result.pop();
        let candidate = format!("{result}…");
        if self::width(fonts, &candidate, size, height) <= width {
            return candidate;
        }
    }
    if self::width(fonts, "…", size, height) <= width {
        "…".into()
    } else {
        String::new()
    }
}
