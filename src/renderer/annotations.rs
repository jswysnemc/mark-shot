mod shadow;
pub use shadow::{selection_chrome_pad, shadow_color_for, visual_pad};
use shadow::{stroke_segment_with_shadow, transforms_for};
mod handles;
use handles::draw_text_box;
mod shapes;
use shapes::{draw_circle, draw_rect, fill_circle, fill_rect};
mod freehand;
use freehand::{draw_highlighter, draw_pen};
pub use freehand::{draw_highlighter_live, draw_pen_active_tail, stroke_pen_segment};
mod handles_outline;
use handles_outline::draw_annotation_handles;
mod shadow_stroke;
use shadow_stroke::stroke_with_shadow;
mod numbered;
use numbered::draw_numerated_arrow;

use super::Painter;
use super::paths::{luminance, normalized_rect, oval_path, rect_bounds, scale_rect};
use super::text::shape_single_line;
use crate::renderer::paths::KAPPA;
use crate::tools::text::render_text_annotation_with_caret;
use crate::types::annotations::{Annotation, AnnotationShape};

use cosmic_text::Editor;
use std::collections::HashMap;
use tiny_skia::{
    Color, FillRule, LineCap, LineJoin, Mask, Paint, PathBuilder, Pixmap, Rect, Stroke, Transform,
};

/// 【标注渲染】【单项正文】将 ann 绘入 p；selected 控制编辑框，text_editors、blurs 提供缓存，active_text_id 指定文字编辑对象，无返回值
pub fn draw_annotation(
    p: &mut Painter,
    ann: &Annotation,
    selected: bool,
    text_editors: &mut HashMap<u64, Editor<'static>>,
    filters: &HashMap<u64, crate::features::filter::raster::Raster>,
    active_text_id: Option<u64>,
) {
    match &ann.shape {
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
        AnnotationShape::Rectangle { start, end, filled } => {
            if let Some(rect) = normalized_rect(*start, *end) {
                if *filled {
                    fill_rect(p, &rect, ann.color, ann.stroke_width, ann.shadow_color);
                } else {
                    draw_rect(p, &rect, ann.color, ann.stroke_width, ann.shadow_color);
                }
            }
        }
        AnnotationShape::Marker {
            start,
            end,
            kind,
            filled,
        } => {
            if let Some(rect) = normalized_rect(*start, *end) {
                crate::features::marker::draw_marker(
                    p,
                    *kind,
                    rect,
                    ann.color,
                    ann.stroke_width,
                    *filled,
                );
            }
        }
        AnnotationShape::Circle { start, end, filled } => {
            if let Some(rect) = normalized_rect(*start, *end) {
                if *filled {
                    fill_circle(p, &rect, ann.color, ann.stroke_width, ann.shadow_color);
                } else {
                    draw_circle(p, &rect, ann.color, ann.stroke_width, ann.shadow_color);
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
    if selected {
        draw_annotation_handles_only(p, ann);
    }
}

/// 【标注渲染】【编辑控制框】仅把 ann 的控制点绘入 p，不绘制正文或修改数据，无返回值
pub fn draw_annotation_handles_only(p: &mut Painter, ann: &Annotation) {
    if ann.shape.line().is_some() {
        super::line_handles::draw(p, ann);
    } else if let AnnotationShape::Redact { filter } = ann.shape {
        super::filter_handles::draw(p, filter);
    } else if matches!(ann.shape, AnnotationShape::Number { .. }) {
        super::number_handles::draw(p, ann);
    } else if matches!(ann.shape, AnnotationShape::Magnifier { .. }) {
        super::magnifier_handles::draw(p, ann);
    } else if matches!(ann.shape, AnnotationShape::Ruler { .. }) {
        super::ruler_overlay::draw_handles(p, ann);
    } else if matches!(ann.shape, AnnotationShape::Text { .. }) {
        draw_text_box(p, &ann.bbox);
    } else {
        draw_annotation_handles(p, &ann.bbox);
    }
}

/// 【标注渲染】【精确外框】将已经包含命中留白的 bbox 外框绘入 p，不重新扩大控制框；无返回值
pub(super) fn draw_exact_frame_handles(p: &mut Painter<'_>, bbox: &Rect) {
    handles_outline::draw_exact_frame(p, bbox);
}
