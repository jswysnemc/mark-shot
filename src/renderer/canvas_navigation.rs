//! 【画布导航】【变换场景】从冻结原图和完整矢量标注绘制当前视图，屏幕控件使用独立设备比例

use super::{Painter, RenderRequest, annotation_stack, annotations};
use crate::editor::canvas_navigation::transform::ViewTransform;
use tiny_skia::Rect;

/// 【画布导航】【背景遮罩】按req与view从冻结原图恢复dirty范围并绘制选区外遮罩，无返回值
fn background(req: &mut RenderRequest<'_>, view: ViewTransform) {
    let native = req.scale;
    let origin = view.origin(req.offset);
    let scale = native * view.scale as f32;
    let transform =
        tiny_skia::Transform::from_translate(-origin.0, -origin.1).post_scale(scale, scale);
    crate::theme::canvas::checkerboard(req.dimmed, native);
    // 1. 【画布导航】【原图采样】完整输出只在视图或选择变化时重采样，局部重绘复用同一像素网格
    req.frozen_image.draw_transformed(req.dimmed, transform);
    super::background_restore::shade(
        &mut Painter::new(req.dimmed).with_scale(native),
        req.selection,
        req.size,
    );
}

/// 【画布导航】【完整绘制】按req、view与background_changed恢复场景，original_text_id排除文字原件，其他叠层参数提供控件，无返回值
pub(super) fn draw(
    req: &mut RenderRequest<'_>,
    view: ViewTransform,
    background_changed: bool,
    original_text_id: Option<u64>,
    annotation_delete: Option<&crate::ui::annotation_selection::delete_cache::Cache>,
    collection: Option<&crate::app::annotation_collection::chrome::Overlay<'_>>,
    region: Option<&crate::editor::region_rendering::State>,
    navigation_cache: Option<&std::cell::RefCell<super::navigation_background::TileCache>>,
) {
    if region.is_some_and(|region| region.native_scene)
        && let Some(cache) = navigation_cache
    {
        super::navigation_background::restore(req, view, background_changed, cache);
    } else {
        if background_changed {
            background(req, view);
        }
        if let Some(dirty) = req.dirty_rect {
            super::blit_rect(req.dimmed, req.canvas, dirty, req.scale);
        } else {
            req.canvas.data_mut().copy_from_slice(req.dimmed.data());
        }
    }
    let origin = view.origin(req.offset);
    let scale = req.scale * view.scale as f32;
    let dirty = req.dirty_rect.and_then(|rect| {
        Rect::from_xywh(
            rect.x() / view.scale as f32,
            rect.y() / view.scale as f32,
            rect.width() / view.scale as f32,
            rect.height() / view.scale as f32,
        )
    });
    if let Some(region) = region.filter(|region| !region.cached_annotations) {
        super::annotation_tiles::draw(req, view, original_text_id, collection, region);
    } else {
        let mut painter = Painter::new(req.canvas)
            .with_scale(scale)
            .with_chrome_scale(1.0 / view.scale as f32)
            .with_ruler_projection(view)
            .with_origin(origin)
            .with_frozen_image(req.frozen_image)
            .with_drawing_bounds(req.drawing_bounds)
            .with_text_caret(req.text_caret_visible)
            .with_text(req.text.font_system, req.text.swash_cache);
        let annotations = if req.annotations_layer_empty {
            &[][..]
        } else {
            req.annotations
        };
        if original_text_id.is_none()
            && let Some((index, pending)) = req.dragged_index.zip(req.pending)
        {
            annotation_stack::draw_dragged(
                &mut painter,
                annotations,
                pending,
                index,
                req.text.editors,
                req.filters,
                req.active_text_id,
                dirty,
            );
        } else if !annotations.is_empty() || req.pending.is_some() {
            // 2. 【画布导航】【文字原件】独立预览替代正在编辑的持久正文，不在原始数组内修改或删除标注
            annotation_stack::draw_ordered(
                &mut painter,
                annotations
                    .iter()
                    .filter(|annotation| Some(annotation.id) != original_text_id)
                    .chain(req.pending),
                req.text.editors,
                req.filters,
                req.active_text_id,
                dirty,
            );
        }
    }
    super::ruler_overlay::draw_clipped_with_window(
        req.canvas,
        req.ruler_labels,
        req.ruler_hover,
        origin,
        scale,
        dirty.as_ref(),
        req.text.font_system,
        req.text.swash_cache,
        crate::features::annotation_style::window::WindowUnits::new(1.0 / view.scale as f32),
    );
    if let Some(laser) = req.laser {
        crate::features::laser_trace::draw::draw_in_window(
            req.canvas,
            laser,
            origin,
            dirty.as_ref(),
            scale,
            std::time::Instant::now(),
            crate::features::annotation_style::window::WindowUnits::new(1.0 / view.scale as f32),
        );
    }
    if let Some(selection) = req.selection {
        super::selection_border_damage::draw(
            req.canvas,
            req.scale,
            req.dirty_rect.copied(),
            selection,
            req.selection_edges,
            req.selection_radius,
        );
        super::selection_handles::draw(
            req.canvas,
            req.scale,
            req.dirty_rect.copied(),
            selection,
            req.selection_edges,
        );
    }
    if req.selected_annotation.is_some() || req.is_pending_selected {
        let mut painter = Painter::new(req.canvas)
            .with_scale(scale)
            .with_origin(origin)
            .with_drawing_bounds(req.drawing_bounds)
            .with_ruler_projection(view)
            .with_chrome_scale(1.0 / view.scale as f32);
        painter.clipped(dirty, |painter| {
            for annotation in req
                .pending
                .filter(|_| req.is_pending_selected)
                .into_iter()
                .chain(
                    req.selected_annotation
                        .and_then(|index| req.annotations.get(index)),
                )
            {
                annotations::draw_annotation_handles_only(painter, annotation);
                if let Some((id, index)) = req.line_point
                    && annotation.id == id
                {
                    super::line_handles::highlight(painter, annotation, index);
                }
            }
        });
    }
    if let Some(collection) = collection {
        super::collection_stack::chrome_with_view(req, collection, view);
    }
    if let Some(cache) = annotation_delete {
        cache.composite(req.canvas, req.dirty_rect.copied());
    }
    if let Some(ocr) = req.ocr_view {
        crate::ocr::draw::draw_ocr_overlay(req.canvas, ocr, origin, dirty.as_ref(), scale);
    }
    if let Some((region, phase)) = req.ocr_scan {
        crate::ocr::draw::draw_ocr_scan(
            req.canvas,
            region,
            phase,
            origin,
            req.icons_cache,
            dirty.as_ref(),
            scale,
        );
    }
}
