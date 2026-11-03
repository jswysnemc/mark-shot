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
    let Some(mut temporary) = Pixmap::new(width, height) else {
        return;
    };
    let index = index.min(annotations.len());
    {
        let mut local = painter.sub(&mut temporary).with_origin(local_origin);
        let mut draw = |annotation: &Annotation| {
            let bounds = annotation.damage_bbox(false);
            if bounds.left() < global.right()
                && bounds.right() > global.left()
                && bounds.top() < global.bottom()
                && bounds.bottom() > global.top()
            {
                annotations::draw_annotation(
                    &mut local,
                    annotation,
                    false,
                    editors,
                    filters,
                    active_text_id,
                );
            }
        };
        // 1. 【标注渲染】【拖动合成】只重绘损坏区域内相交的标注，插入草稿不复制整个标注数组
        for (position, annotation) in annotations.iter().enumerate() {
            if position == index {
                draw(pending);
            }
            draw(annotation);
        }
        if index == annotations.len() {
            draw(pending);
        }
    }
    // 2. 【标注渲染】【拖动合成】透明图像与半透明描边只合成一次，不覆盖较新的标注或重复叠加已有图层
    painter.draw_pixmap(x as i32, y as i32, &temporary);
}
