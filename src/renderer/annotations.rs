mod body;
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
#[cfg(test)]
mod selection_chrome_tests;

use super::Painter;
use super::paths::{luminance, normalized_rect, oval_path, rect_bounds, scale_rect};
use super::text::shape_single_line;
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
    let painted = if matches!(ann.shape, AnnotationShape::Text { .. }) {
        p.fonts.as_mut().map(|fonts| {
            let editor = crate::tools::text::ensure_text_editor(ann, text_editors, fonts.system);
            crate::tools::text::text_bounds(ann, editor)
        })
    } else {
        None
    };
    super::annotation_rotation::draw(p, ann, painted, active_text_id == Some(ann.id), |p| {
        body::draw(p, ann, text_editors, filters, active_text_id);
    });
    if selected {
        draw_annotation_handles_only(p, ann);
    }
}

/// 【标注渲染】【编辑控制框】仅把 ann 的控制点绘入 p，不绘制正文或修改数据，无返回值
pub fn draw_annotation_handles_only(p: &mut Painter, ann: &Annotation) {
    if let Some(corners) = ann.generic_frame_corners() {
        let draw = crate::ui::annotation_selection::draw::frame;
        draw(p, corners, 1.0);
        for point in crate::features::annotation_selection::chrome::handles(corners) {
            crate::ui::annotation_selection::draw::square(p, point, 1.0);
        }
        if let Some((top, handle)) = ann.generic_rotation_handle() {
            crate::ui::annotation_selection::draw::rotation(p, top, handle, 1.0);
        }
        if ann.shape.line().is_some() {
            super::line_handles::anchors(p, ann);
        }
    } else if ann.shape.line().is_some() {
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
        if let Some(bounds) = crate::features::annotation_selection::chrome::bounds(ann) {
            draw_annotation_handles(p, &bounds);
        }
    }
}

/// 【标注渲染】【精确外框】将已经包含命中留白的 bbox 外框绘入 p，不重新扩大控制框；无返回值
pub(super) fn draw_exact_frame_handles(p: &mut Painter<'_>, bbox: &Rect) {
    handles_outline::draw_exact_frame(p, bbox);
}
