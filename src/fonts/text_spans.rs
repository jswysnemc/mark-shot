//! 【文字字体】【缺字片段】按系统字体优先级处理缺字，不让字库枚举顺序决定中文与其他文字的字体

use cosmic_text::{Attrs, AttrsList, Buffer, Family, FontSystem, Shaping};
use unicode_segmentation::UnicodeSegmentation;

/// 【文字字体】【正文填充】将 text 按 attrs 主字体分段写入 buffer；fonts 提供字形，仅缺字片段使用系统回退，保留字素与统一样式，无返回值
pub fn set_text(buffer: &mut Buffer, fonts: &mut FontSystem, text: &str, attrs: &Attrs<'_>) {
    let Family::Name(family) = attrs.family else {
        buffer.set_text(text, attrs, Shaping::Advanced, None);
        return;
    };
    let id = fonts.db().query(&cosmic_text::fontdb::Query {
        families: &[Family::Name(family)],
        weight: attrs.weight,
        stretch: attrs.stretch,
        style: attrs.style,
    });
    let primary = id.and_then(|id| fonts.get_font(id, attrs.weight));
    let mut fallbacks: Vec<(String, std::sync::Arc<cosmic_text::Font>)> = Vec::new();
    // 1. 【文字字体】【段落保留】先用普通正文接口保留末尾空行，再按段落覆盖缺字片段的字体
    buffer.set_text(text, attrs, Shaping::Advanced, None);
    for line in &mut buffer.lines {
        let mut spans: Vec<(usize, usize, String)> = Vec::new();
        for (start, grapheme) in line.text().grapheme_indices(true) {
            let missing = grapheme.chars().find(|character| {
                !character.is_control()
                    && *character != '\u{200d}'
                    && *character != '\u{fe0f}'
                    && primary
                        .as_ref()
                        .is_some_and(|font| font.as_swash().charmap().map(*character) == 0)
            });
            let resolved = missing.map_or_else(
                || family.to_owned(),
                |character| {
                    if let Some((resolved, _)) = fallbacks
                        .iter()
                        .find(|(_, font)| font.as_swash().charmap().map(character) != 0)
                    {
                        return resolved.clone();
                    }
                    let resolved = super::text_faces::fallback(
                        fonts,
                        family,
                        attrs.weight.0 >= 600,
                        attrs.style == cosmic_text::Style::Italic,
                        character,
                    );
                    let query = cosmic_text::fontdb::Query {
                        families: &[Family::Name(&resolved)],
                        weight: attrs.weight,
                        stretch: attrs.stretch,
                        style: attrs.style,
                    };
                    if let Some(font) = fonts
                        .db()
                        .query(&query)
                        .and_then(|id| fonts.get_font(id, attrs.weight))
                    {
                        fallbacks.push((resolved.clone(), font));
                    }
                    resolved
                },
            );
            let end = start + grapheme.len();
            if let Some(previous) = spans.last_mut()
                && previous.2 == resolved
            {
                previous.1 = end;
            } else {
                spans.push((start, end, resolved));
            }
        }
        // 2. 【文字字体】【字素连续】相邻同字体片段合并，避免打断连字、组合字符与双向排版
        let mut list = AttrsList::new(attrs);
        for (start, end, resolved) in spans {
            if resolved != family {
                list.add_span(start..end, &attrs.clone().family(Family::Name(&resolved)));
            }
        }
        line.set_attrs_list(list);
    }
}
