//! 【截图绘制】【稳定场景】原有恒等视图的缓存正文、选择控件和动态叠层

use super::*;

/// 【截图绘制】【场景合成】按req、annotation_delete与collection绘制稳定视图的背景和画布正文，无返回值
pub(super) fn draw(
    req: &mut RenderRequest<'_>,
    annotation_delete: Option<&crate::ui::annotation_selection::delete_cache::Cache>,
    collection: Option<&crate::app::annotation_collection::chrome::Overlay<'_>>,
    original_text_id: Option<u64>,
    region: Option<&crate::editor::region_rendering::State>,
) {
    let s = req.scale;
    if region.is_none() && req.selection_dirty {
        update_dimming_delta(
            req.dimmed,
            req.prev_selection,
            req.selection,
            req.selection_edges,
            s,
            req.selection_radius,
        );
    }
    let dirty_rect = req.dirty_rect;

    if let Some(region) = region {
        background_restore::draw(req);
        if region.cached_annotations {
            if collection.as_ref().is_some_and(|overlay| overlay.dragging) {
                collection_stack::bodies(req);
            } else {
                annotation_stack::draw_layers(req);
            }
        } else {
            annotation_tiles::draw(
                req,
                Default::default(),
                original_text_id,
                collection,
                region,
            );
        }
    } else {
        if let Some(dirty) = dirty_rect {
            blit_rect(req.dimmed, req.canvas, dirty, s);
        } else {
            req.canvas.data_mut().copy_from_slice(req.dimmed.data());
        }

        if collection.as_ref().is_some_and(|overlay| overlay.dragging) {
            collection_stack::bodies(req);
        } else {
            annotation_stack::draw_layers(req);
        }
    }

    ruler_overlay::draw_clipped(
        req.canvas,
        req.ruler_labels,
        req.ruler_hover,
        req.offset,
        s,
        dirty_rect,
        req.text.font_system,
        req.text.swash_cache,
    );

    if let Some(laser) = req.laser {
        crate::features::laser_trace::draw(
            req.canvas,
            laser,
            req.offset,
            dirty_rect,
            s,
            std::time::Instant::now(),
        );
    }

    if let Some(sel) = req.selection {
        selection_border_damage::draw(
            req.canvas,
            s,
            dirty_rect.copied(),
            sel,
            req.selection_edges,
            req.selection_radius,
        );
        super::selection_handles::draw(
            req.canvas,
            s,
            dirty_rect.copied(),
            sel,
            req.selection_edges,
        );
    }

    if let Some(rect) = req.hovered_region {
        Painter::new(req.canvas)
            .with_scale(s)
            .with_origin(req.offset)
            .clipped(dirty_rect.copied(), |painter| {
                crate::app::region_detection::draw::region(painter, *rect);
            });
    }

    if let Some(pending) = req.pending
        && req.is_pending_selected
    {
        Painter::new(req.canvas)
            .with_scale(s)
            .with_origin(req.offset)
            .with_drawing_bounds(req.drawing_bounds)
            .clipped(dirty_rect.copied(), |painter| {
                annotations::draw_annotation_handles_only(painter, pending);
                if let Some((id, index)) = req.line_point
                    && pending.id == id
                {
                    line_handles::highlight(painter, pending, index);
                }
            });
    }

    // Dynamic selection chrome: handles for the currently selected annotation
    if let Some(idx) = req.selected_annotation
        && let Some(ann) = req.annotations.get(idx)
    {
        Painter::new(req.canvas)
            .with_scale(s)
            .with_origin(req.offset)
            .with_drawing_bounds(req.drawing_bounds)
            .clipped(dirty_rect.copied(), |painter| {
                annotations::draw_annotation_handles_only(painter, ann);
                if let Some((id, index)) = req.line_point
                    && ann.id == id
                {
                    line_handles::highlight(painter, ann, index);
                }
            });
    }

    if let Some(overlay) = collection.as_ref() {
        collection_stack::chrome(req, overlay);
    }
    if let Some(cache) = annotation_delete {
        cache.composite(req.canvas, dirty_rect.copied());
    }
    // Clipped to the dirty rect: everything the overlay paints is translucent,
    // so anything drawn outside the area just restored from `dimmed` would
    // stack a second layer on top of last frame's.
    if let Some(view) = req.ocr_view {
        crate::ocr::draw::draw_ocr_overlay(req.canvas, view, req.offset, dirty_rect, s);
    }

    if let Some((region, phase)) = req.ocr_scan {
        crate::ocr::draw::draw_ocr_scan(
            req.canvas,
            region,
            phase,
            req.offset,
            req.icons_cache,
            dirty_rect,
            s,
        );
    }
}
