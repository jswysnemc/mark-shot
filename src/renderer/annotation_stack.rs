//! 【标注渲染】【图层顺序】组合缓存与草稿，已有标注拖动时按原始顺序重绘可见区域

use std::collections::HashMap;

use cosmic_text::Editor;
use tiny_skia::{Pixmap, PixmapPaint, Rect, Transform};

use super::{Painter, RenderRequest, annotations, rect_bounds, scale_rect};
use crate::types::{Annotation, AnnotationShape};

/// 【标注渲染】【正文组合】把 req 的持久图层与当前草稿绘入已经恢复背景的画布；拖动时保留原有层叠顺序，无返回值
pub(super) fn draw_layers(req: &mut RenderRequest<'_>) {
    if let Some((index, pending)) = req.dragged_index.zip(req.pending) {
        let text = &mut req.text;
        draw_dragged(
            &mut Painter::new(req.canvas)
                .with_scale(req.scale)
                .with_text_caret(req.text_caret_visible)
                .with_frozen_image(req.frozen_image)
                .with_drawing_bounds(req.drawing_bounds)
                .with_origin(req.offset)
                .with_text(text.font_system, text.swash_cache),
            req.annotations,
            pending,
            index,
            text.editors,
            req.filters,
            req.active_text_id,
            req.dirty_rect.copied(),
        );
        return;
    }
    let annotations = if req.annotations_layer_empty {
        &[][..]
    } else {
        req.annotations
    };
    if annotations
        .iter()
        .chain(req.pending)
        .any(|annotation| matches!(annotation.shape, AnnotationShape::Highlighter { .. }))
    {
        // 1. 【荧光笔】【背景合成】乘法混合必须读取实际底图，按原顺序绘制当前损坏区域，透明缓存不能保存此效果
        draw_ordered(
            &mut Painter::new(req.canvas)
                .with_scale(req.scale)
                .with_text_caret(req.text_caret_visible)
                .with_frozen_image(req.frozen_image)
                .with_drawing_bounds(req.drawing_bounds)
                .with_origin(req.offset)
                .with_text(req.text.font_system, req.text.swash_cache),
            annotations.iter().chain(req.pending),
            req.text.editors,
            req.filters,
            req.active_text_id,
            req.dirty_rect.copied(),
        );
        return;
    }
    if !req.annotations_layer_empty {
        if let Some(dirty) = req.dirty_rect {
            super::dimming::blit_annotations(req.annotations_layer, req.canvas, dirty, req.scale);
        } else {
            req.canvas.draw_pixmap(
                0,
                0,
                req.annotations_layer.as_ref(),
                &PixmapPaint::default(),
                Transform::identity(),
                None,
            );
        }
    }
    let Some(pending) = req.pending else { return };
    let text = &mut req.text;
    let mut painter = Painter::new(req.canvas)
        .with_scale(req.scale)
        .with_text_caret(req.text_caret_visible)
        .with_frozen_image(req.frozen_image)
        .with_drawing_bounds(req.drawing_bounds)
        .with_origin(req.offset)
        .with_text(text.font_system, text.swash_cache);
    if !req.is_pending_selected
        && pending.rotation_degrees == 0.0
        && let AnnotationShape::Pen { points } = &pending.shape
    {
        annotations::draw_pen_active_tail(
            &mut painter,
            points,
            req.pending_pen_baked,
            pending.color,
            pending.stroke_width,
        );
    } else if !req.is_pending_selected
        && pending.rotation_degrees == 0.0
        && let AnnotationShape::Highlighter { points, flat_ends } = &pending.shape
    {
        annotations::draw_highlighter_live(
            &mut painter,
            points,
            *flat_ends,
            pending.color,
            pending.stroke_width,
            req.dirty_rect,
        );
    } else {
        painter.clipped(req.dirty_rect.copied(), |painter| {
            annotations::draw_annotation(
                painter,
                pending,
                false,
                text.editors,
                req.filters,
                req.active_text_id,
            )
        });
    }
}

/// 【标注渲染】【拖动合成】把 pending 插入 annotations 的 index 位置并合成到 painter；editors、blurs、active_text_id 提供内容，dirty 限制本地范围，无返回值
pub fn draw_dragged(
    painter: &mut Painter<'_>,
    annotations: &[Annotation],
    pending: &Annotation,
    index: usize,
    editors: &mut HashMap<u64, Editor<'static>>,
    filters: &HashMap<u64, crate::features::filter::raster::Raster>,
    active_text_id: Option<u64>,
    dirty: Option<Rect>,
) {
    let index = index.min(annotations.len());
    draw_ordered(
        painter,
        annotations[..index]
            .iter()
            .chain(std::iter::once(pending))
            .chain(annotations[index..].iter()),
        editors,
        filters,
        active_text_id,
        dirty,
    );
}

/// 【标注渲染】【有序合成】按ordered顺序把相交成员绘入painter，editors、filters与active_text_id提供内容，dirty限制范围，无返回值
pub(crate) fn draw_ordered<'a, I>(
    painter: &mut Painter<'_>,
    ordered: I,
    editors: &mut HashMap<u64, Editor<'static>>,
    filters: &HashMap<u64, crate::features::filter::raster::Raster>,
    active_text_id: Option<u64>,
    dirty: Option<Rect>,
) where
    I: IntoIterator<Item = &'a Annotation>,
    I::IntoIter: Clone,
{
    let ordered = ordered.into_iter();
    let multiply = ordered
        .clone()
        .any(|annotation| matches!(annotation.shape, AnnotationShape::Highlighter { .. }));
    let size = painter.size();
    let Some(viewport) = Rect::from_xywh(0.0, 0.0, size.0, size.1) else {
        return;
    };
    let Some(area) = dirty
        .unwrap_or(viewport)
        .intersect(&viewport)
        .filter(|rect| !rect.is_empty())
    else {
        return;
    };
    let scale = painter.scale();
    let Some(pixels) = scale_rect(&area, scale) else {
        return;
    };
    let Some((x, y, width, height)) =
        rect_bounds(&pixels, painter.canvas().width(), painter.canvas().height())
    else {
        return;
    };
    let origin = painter.origin();
    let local_origin = (origin.0 + x as f32 / scale, origin.1 + y as f32 / scale);
    let Some(global) = Rect::from_xywh(
        local_origin.0,
        local_origin.1,
        width as f32 / scale,
        height as f32 / scale,
    ) else {
        return;
    };
    if multiply {
        // 1. 【荧光笔】【有序绘制】乘法混合读取当前局部底图，完成后直接覆盖当前损坏范围
        painter.clipped(dirty, |local| {
            draw_visible(local, ordered, global, editors, filters, active_text_id)
        });
    } else if let Some(mut temporary) = Pixmap::new(width, height) {
        // 2. 【标注渲染】【有序绘制】普通标注继续使用与持久缓存相同的透明合成，保证草稿和提交后的舍入一致
        draw_visible(
            &mut painter.sub(&mut temporary).with_origin(local_origin),
            ordered,
            global,
            editors,
            filters,
            active_text_id,
        );
        painter.draw_pixmap(x as i32, y as i32, &temporary);
    }
}

/// 【标注渲染】【可见成员】按ordered顺序将与global相交的实际正文绘入local，editors、filters和active_text_id提供内容，无返回值
pub(super) fn draw_visible<'a>(
    local: &mut Painter<'_>,
    ordered: impl Iterator<Item = &'a Annotation>,
    global: Rect,
    editors: &mut HashMap<u64, Editor<'static>>,
    filters: &HashMap<u64, crate::features::filter::raster::Raster>,
    active_text_id: Option<u64>,
) {
    for annotation in ordered {
        let bounds = super::annotation_style::damage(
            annotation,
            local.window_units(),
            local.drawing_bounds(),
            local.ruler_projection(),
        );
        if bounds.left() < global.right()
            && bounds.right() > global.left()
            && bounds.top() < global.bottom()
            && bounds.bottom() > global.top()
        {
            annotations::draw_annotation(
                local,
                annotation,
                false,
                editors,
                filters,
                active_text_id,
            );
        }
    }
}
