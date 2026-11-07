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
    let primary_weight = matched_weight(fonts, attrs);
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
            let mut actual = attrs.clone().family(Family::Name(&resolved));
            actual.weight = if resolved == family {
                primary_weight
            } else {
                matched_weight(fonts, &actual)
            };
            // 3. 【文字字体】【字重隔离】默认属性保留用户请求，实际片段使用本家族可用字重，避免加载其他家族后改变回退结果
            if resolved != family || actual.weight != attrs.weight {
                list.add_span(start..end, &actual);
            }
        }
        line.set_attrs_list(list);
    }
}

/// 【文字字体】【字重匹配】返回 attrs 家族实际可用的字重；可变字体覆盖请求值时保留原值，不修改文档默认格式
fn matched_weight(fonts: &mut FontSystem, attrs: &Attrs<'_>) -> cosmic_text::Weight {
    let Some(id) = fonts.db().query(&cosmic_text::fontdb::Query {
        families: &[attrs.family],
        weight: attrs.weight,
        stretch: attrs.stretch,
        style: attrs.style,
    }) else {
        return attrs.weight;
    };
    let weight = fonts.db().face(id).unwrap().weight;
    if weight == attrs.weight {
        return weight;
    }
    let variable = fonts.get_font(id, attrs.weight).is_some_and(|font| {
        font.as_swash().variations().into_iter().any(|axis| {
            axis.tag() == u32::from_be_bytes(*b"wght")
                && (axis.min_value()..=axis.max_value()).contains(&f32::from(attrs.weight.0))
        })
    });
    if variable { attrs.weight } else { weight }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 【文字字体测试】【加载顺序】缺少六百字重时仍使用 Inter，新增其他家族的精确字重不能改变字形和宽度
    #[test]
    fn missing_weight_stays_in_family_after_loading_another_family() {
        let mut fonts = crate::fonts::bundled();
        let attrs = Attrs::new()
            .family(Family::Name("Inter"))
            .weight(cosmic_text::Weight(600));
        let shape = |fonts: &mut FontSystem| {
            let mut buffer = Buffer::new(fonts, cosmic_text::Metrics::new(27.0, 41.0));
            set_text(&mut buffer, fonts, "original", &attrs);
            buffer.shape_until_scroll(fonts, false);
            assert_eq!(buffer.lines[0].attrs_list().defaults().weight, attrs.weight);
            buffer
                .layout_runs()
                .flat_map(|run| run.glyphs.iter())
                .map(|glyph| {
                    let face = fonts.db().face(glyph.font_id).unwrap();
                    assert_eq!(face.families[0].0, "Inter");
                    (glyph.font_id, glyph.glyph_id, glyph.x, glyph.w)
                })
                .collect::<Vec<_>>()
        };
        let before = shape(&mut fonts);
        // 1. 【文字字体测试】【隔离字库】复制内置字形元数据模拟其他家族六百字重，无需系统字体或全局 Fontconfig 配置
        let mut other = fonts
            .db()
            .faces()
            .find(|face| face.weight == cosmic_text::Weight::BOLD)
            .unwrap()
            .clone();
        other.families[0].0 = "Noto Sans".into();
        other.weight = cosmic_text::Weight(600);
        fonts.db_mut().push_face_info(other);
        assert_eq!(shape(&mut fonts), before);
    }
}
