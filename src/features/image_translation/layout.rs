//! 【图片翻译】【覆盖排版】原文行高决定字号，逐步缩小、向下避让及省略，全文和显示内容保持独立

use super::{BoxRect, SourceBlock, TranslatedBlock, TranslatedLine, colors, shaping};
use cosmic_text::FontSystem;
use tiny_skia::Pixmap;

/// 【图片翻译】【显示高度】按 fitted 返回首行顶边到末行底边总高度，空行集仍占一行字体高度
fn height(fitted: &shaping::Text) -> f64 {
    (fitted.lines.len().max(1) - 1) as f64 * fitted.advance + fitted.height
}

/// 【图片翻译】【字号适配】按 fonts、text、source 和 available 返回原版最大可用整像素字号，最小九像素仍过高时保留结果
fn fit(fonts: &mut FontSystem, text: &str, source: &SourceBlock, available: f64) -> shaping::Text {
    let start = (source.line_height * 0.78).round().clamp(9.0, 96.0);
    let mut size = start;
    loop {
        let pixels = size.round().max(9.0);
        let fitted = shaping::shape(
            fonts,
            text,
            pixels as f32,
            source.rect.width(),
            source.line_pitch,
            start,
        );
        if height(&fitted) <= available || pixels <= 9.0 {
            return fitted;
        }
        size = (size * 0.92).min(pixels - 1.0);
    }
}

/// 【图片翻译】【下方避让】按 blocks、index 和 bounds 返回横向重叠后续段落前一像素或图片底边
fn below(blocks: &[SourceBlock], index: usize, bounds: BoxRect) -> f64 {
    let rect = blocks[index].rect;
    blocks
        .iter()
        .enumerate()
        .filter(|(other, block)| {
            *other != index
                && block.rect.top() >= rect.bottom()
                && block.rect.right() > rect.left()
                && block.rect.left() < rect.right()
        })
        .map(|(_, block)| block.rect.top() - 1.0)
        .fold(bounds.bottom(), f64::min)
}

/// 【图片翻译】【段落排版】按 image、sources、texts 和 fonts 返回可显示覆盖层，配色只来自原图，行宽采用实际字形宽度
pub fn translated(
    image: &Pixmap,
    sources: &[SourceBlock],
    texts: &[String],
    fonts: &mut FontSystem,
) -> Vec<TranslatedBlock> {
    let bounds = BoxRect::new(
        0.0,
        0.0,
        f64::from(image.width()),
        f64::from(image.height()),
    );
    let mut result = Vec::new();
    for (index, (source, text)) in sources.iter().zip(texts).enumerate() {
        let text = text.trim();
        if text.is_empty()
            || !source.rect.valid()
            || !source.line_height.is_finite()
            || !source.line_pitch.is_finite()
        {
            continue;
        }
        shaping::prepare(fonts, text);
        let padding = (source.line_height * 0.12).clamp(1.0, 6.0);
        let mut plate = source.rect.expanded(padding).intersect(bounds);
        if !plate.valid() {
            continue;
        }
        let mut fitted = fit(fonts, text, source, plate.height());
        let mut needed = height(&fitted);
        // 1. 【图片翻译】【尾部避让】最小字号放不下时延伸底板，仍不足的行采用省略显示，完整译文保留
        if needed > plate.height() {
            let limit = plate.bottom().max(below(sources, index, bounds));
            plate.0[3] = limit.min(plate.top() + needed) - plate.top();
            let capacity = (((plate.height() - fitted.height) / fitted.advance).floor() as isize
                + 1)
            .max(1) as usize;
            if fitted.lines.len() > capacity {
                fitted.lines.truncate(capacity);
                let last = fitted.lines.last_mut().unwrap();
                *last =
                    shaping::elide(fonts, last, fitted.size, fitted.height, source.rect.width());
                needed = height(&fitted);
            }
        }
        let top = if fitted.lines.len() <= 1 {
            source.rect.top() + source.rect.height() / 2.0 - needed / 2.0
        } else {
            source.rect.top()
        };
        let top = top.clamp(plate.top(), plate.top().max(plate.bottom() - needed));
        let colors = colors::sample(image, source.rect, source.line_height);
        let mut lines = Vec::new();
        for (row, text) in fitted.lines.iter().enumerate() {
            let y = top + row as f64 * fitted.advance;
            let width = shaping::width(fonts, text, fitted.size, fitted.height)
                .clamp(1.0, (plate.right() - source.rect.left()).max(1.0));
            lines.push(TranslatedLine {
                text: text.clone(),
                rect: BoxRect::new(source.rect.left(), y, width, fitted.height),
                baseline: y + fitted.ascent,
            });
        }
        result.push(TranslatedBlock {
            source: source.rect,
            plate,
            background: colors.background,
            foreground: colors.foreground,
            font_size: fitted.size,
            text: text.to_owned(),
            lines,
        });
    }
    result
}
