//! 【标注渲染】【正文实现】分派各类正文绘制，选框和旋转由外层组合

use super::*;

/// 【标注渲染】【正文分派】将 ann 绘入 p；text_editors、filters 提供缓存，active_text_id 控制文字编辑，无返回值
pub(super) fn draw(
    p: &mut Painter,
    ann: &Annotation,
    text_editors: &mut HashMap<u64, Editor<'static>>,
    filters: &HashMap<u64, crate::features::filter::raster::Raster>,
    active_text_id: Option<u64>,
) {
    let width = p.window_units().stroke(ann.stroke_width, 1.5, None);
    match &ann.shape {
        AnnotationShape::ColorSample { sample } => {
            crate::features::color_sample::draw::draw(p, *sample)
        }
        AnnotationShape::Number { mark } => {
            crate::features::number::draw::draw(p, *mark, ann.stroke_width, ann.color);
        }
        AnnotationShape::Magnifier { magnifier } => {
            crate::features::magnifier::draw::draw(p, *magnifier, ann.stroke_width, ann.color);
        }
        AnnotationShape::Ruler { measurement } => {
            crate::features::ruler::body_draw::draw(p, *measurement, ann.stroke_width, ann.color);
        }
        AnnotationShape::Arrow {
            line,
            style,
            line_style,
        } => {
            crate::features::arrow::draw::draw(
                p,
                line,
                *style,
                *line_style,
                ann.stroke_width,
                ann.color,
            );
        }
        AnnotationShape::Rectangle {
            start,
            end,
            filled,
            radius,
            ..
        } => {
            if let Some(rect) = normalized_rect(*start, *end) {
                let radius = ann.style_units().canvas(*radius);
                if *filled {
                    fill_rect(p, &rect, ann.color, width, ann.shadow_color, radius);
                } else {
                    draw_rect(p, &rect, ann.color, width, ann.shadow_color, radius);
                }
            }
        }
        AnnotationShape::Marker {
            start,
            end,
            kind,
            filled,
            radius,
        } => {
            if let Some(rect) = normalized_rect(*start, *end) {
                if *kind == crate::types::MarkerShape::Square {
                    let radius = ann.style_units().canvas(*radius);
                    if *filled {
                        fill_rect(p, &rect, ann.color, width, ann.shadow_color, radius);
                    } else {
                        draw_rect(p, &rect, ann.color, width, ann.shadow_color, radius);
                    }
                } else {
                    crate::features::marker::draw_marker(p, *kind, rect, ann.color, width, *filled);
                }
            }
        }
        AnnotationShape::Circle {
            start, end, filled, ..
        } => {
            if let Some(rect) = normalized_rect(*start, *end) {
                if *filled {
                    fill_circle(p, &rect, ann.color, width, ann.shadow_color);
                } else {
                    draw_circle(p, &rect, ann.color, width, ann.shadow_color);
                }
            }
        }
        AnnotationShape::Line { line, style } => {
            crate::features::line::draw::draw(p, line, *style, ann.stroke_width, ann.color);
        }
        AnnotationShape::Pen { points } => {
            draw_pen(p, points, ann.color, ann.stroke_width, ann.shadow_color);
        }
        AnnotationShape::Redact { filter } => {
            crate::features::filter::draw::draw(
                p,
                *filter,
                ann.color.alpha(),
                filters.get(&ann.id),
            );
        }
        AnnotationShape::Highlighter { points, flat_ends } => {
            draw_highlighter(p, points, *flat_ends, ann.color, ann.stroke_width);
        }
        AnnotationShape::Text { .. } => {
            let is_editing = active_text_id == Some(ann.id);
            let scale = p.scale();
            let pixel_phase = p.pixel_phase();
            if let Some(fonts) = p.fonts.as_mut() {
                let editor =
                    crate::tools::text::ensure_text_editor(ann, text_editors, fonts.system);
                let mut pixmap_mut = p.canvas.as_mut();
                render_text_annotation_with_caret(
                    ann,
                    editor,
                    fonts.system,
                    fonts.cache,
                    &mut pixmap_mut,
                    p.origin,
                    scale,
                    is_editing,
                    p.text_caret_visible,
                    Some(pixel_phase),
                );
            }
        }
        AnnotationShape::NumeratedArrow { start, end, number } => {
            draw_numerated_arrow(
                p,
                *start,
                *end,
                *number,
                ann.color,
                ann.stroke_width,
                ann.shadow_color,
            );
        }
    }
}
