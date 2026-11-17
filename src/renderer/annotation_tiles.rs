//! 【标注渲染】【区域合成】按整数设备像素分块绘制正文，透明图层和真实底图分别保持原有混合顺序

use super::{Painter, RenderRequest, annotation_stack, annotations, background_restore};
use crate::editor::region_rendering::State;
use crate::{editor::canvas_navigation::transform::ViewTransform, types::Annotation};
use tiny_skia::{Color, IntRect, Pixmap, Rect};

const SIDE: u32 = 512;

/// 【标注渲染】【背景复制】按position将canvas对应像素写入tile，画布外填透明，无返回值
fn read(canvas: &Pixmap, tile: &mut Pixmap, position: (u32, u32)) {
    tile.fill(Color::TRANSPARENT);
    let width = SIDE.min(canvas.width() - position.0) as usize * 4;
    let height = SIDE.min(canvas.height() - position.1) as usize;
    let stride = canvas.width() as usize * 4;
    for row in 0..height {
        let source = (position.1 as usize + row) * stride + position.0 as usize * 4;
        let target = row * SIDE as usize * 4;
        tile.data_mut()[target..target + width]
            .copy_from_slice(&canvas.data()[source..source + width]);
    }
}

/// 【标注渲染】【像素交付】按position与area将tile写回canvas，blend指定透明层合成或底图覆盖，无返回值
fn write(canvas: &mut Pixmap, tile: &Pixmap, position: (u32, u32), area: IntRect, blend: bool) {
    let stride = canvas.width() as usize * 4;
    let bytes = area.width() as usize * 4;
    for y in area.top()..area.bottom() {
        let source = (y as u32 - position.1) as usize * SIDE as usize * 4
            + (area.left() as u32 - position.0) as usize * 4;
        let target = y as usize * stride + area.left() as usize * 4;
        let source = &tile.data()[source..source + bytes];
        let target = &mut canvas.data_mut()[target..target + bytes];
        if !blend {
            crate::utils::transparent_pixmap::copy_changed(target, source);
            continue;
        }
        super::annotation_tile_blend::blend(source, target);
    }
}

/// 【标注渲染】【可见相交】返回bounds是否与global相交，用于跳过不会贡献像素的正文和画笔片段
fn intersects(bounds: Rect, global: Rect) -> bool {
    bounds.left() < global.right()
        && bounds.right() > global.left()
        && bounds.top() < global.bottom()
        && bounds.bottom() > global.top()
}

/// 【标注渲染】【固定块遍历】按req与view恢复每个损坏块后执行body，blend决定透明合成，返回值为空
fn tiles(
    req: &mut RenderRequest<'_>,
    view: ViewTransform,
    blend: bool,
    mut body: impl FnMut(
        &mut Painter<'_>,
        Rect,
        &mut std::collections::HashMap<u64, cosmic_text::Editor<'static>>,
    ) -> bool,
) {
    let Some(area) = background_restore::damage(req.canvas, req.dirty_rect, req.scale) else {
        return;
    };
    let Some(mut tile) = Pixmap::new(SIDE, SIDE) else {
        return;
    };
    let scale = req.scale * view.scale as f32;
    let origin = view.origin(req.offset);
    let mut painter = Painter::new(req.canvas)
        .with_scale(scale)
        .with_origin(origin)
        .with_chrome_scale(1.0 / view.scale as f32)
        .with_ruler_projection(view)
        .with_frozen_image(req.frozen_image)
        .with_drawing_bounds(req.drawing_bounds)
        .with_text_caret(req.text_caret_visible)
        .with_text(req.text.font_system, req.text.swash_cache);
    for y in (area.top() as u32 / SIDE..=(area.bottom() as u32 - 1) / SIDE).map(|y| y * SIDE) {
        for x in (area.left() as u32 / SIDE..=(area.right() as u32 - 1) / SIDE).map(|x| x * SIDE) {
            if blend {
                tile.fill(Color::TRANSPARENT);
            } else {
                read(painter.canvas(), &mut tile, (x, y));
            }
            // 1. 【标注渲染】【固定相位】子画布继承完整视口相位，局部原点只增加整数物理像素
            let local_origin = (origin.0 + x as f32 / scale, origin.1 + y as f32 / scale);
            let bounds = IntRect::from_xywh(x as i32, y as i32, SIDE, SIDE).unwrap();
            let Some(visible) = bounds.intersect(&area) else {
                continue;
            };
            let global = Rect::from_xywh(
                origin.0 + visible.left() as f32 / scale,
                origin.1 + visible.top() as f32 / scale,
                visible.width() as f32 / scale,
                visible.height() as f32 / scale,
            )
            .unwrap();
            let painted = body(
                &mut painter.sub(&mut tile).with_origin(local_origin),
                global,
                req.text.editors,
            );
            // 2. 【聚焦滤镜】【透明块跳过】透明孔洞没有正文贡献，不扫描或交付完整零像素块
            if !painted {
                continue;
            }
            // 3. 【标注渲染】【损坏交付】相交像素只合成一次，块内其他像素不写回最终画布
            write(painter.canvas(), &tile, (x, y), visible, blend);
        }
    }
}

/// 【标注渲染】【有序正文】按req、view与ordered绘制完整有序正文，multiply控制真实底图混合，无返回值
fn ordered(
    req: &mut RenderRequest<'_>,
    view: ViewTransform,
    ordered: &[&Annotation],
    multiply: bool,
) {
    if ordered.is_empty() {
        return;
    }
    let filters = req.filters;
    let active_text_id = req.active_text_id;
    tiles(req, view, !multiply, |painter, global, editors| {
        annotation_stack::draw_visible(
            painter,
            ordered.iter().copied(),
            global,
            editors,
            filters,
            active_text_id,
        )
    });
}

/// 【标注渲染】【实时草稿】按req将当前草稿直接绘入恢复后的局部底图，保持旧正文与草稿的两次合成顺序，无返回值
fn pending(req: &mut RenderRequest<'_>) {
    let Some(pending) = req.pending else { return };
    if matches!(pending.shape, crate::types::AnnotationShape::Ruler { .. }) {
        super::pending_ruler::draw(req, pending);
        return;
    }
    let filters = req.filters;
    let active_text_id = req.active_text_id;
    let tail = !req.is_pending_selected && pending.rotation_degrees == 0.0;
    let baked = req.pending_pen_baked;
    tiles(
        req,
        Default::default(),
        false,
        |painter, global, editors| {
            if !intersects(pending.damage_bbox(false), global) {
                return false;
            }
            if crate::features::filter::focus::inside_hole(pending, global, 2.0 / painter.scale()) {
                return false;
            }
            if tail && let crate::types::AnnotationShape::Pen { points } = &pending.shape {
                annotations::draw_pen_active_tail(
                    painter,
                    points,
                    baked,
                    pending.color,
                    pending.stroke_width,
                );
            } else if crate::features::filter::focus::enabled(pending) {
                painter.clipped_scene(Some(global), |painter| {
                    annotations::draw_annotation(
                        painter,
                        pending,
                        false,
                        editors,
                        filters,
                        active_text_id,
                    );
                });
            } else {
                annotations::draw_annotation(
                    painter,
                    pending,
                    false,
                    editors,
                    filters,
                    active_text_id,
                );
            }
            true
        },
    );
}

/// 【标注渲染】【区域场景】按req、view、original_text_id、collection与region合成正文，保留拖动顺序和文字原件排除，无返回值
pub(super) fn draw(
    req: &mut RenderRequest<'_>,
    view: ViewTransform,
    original_text_id: Option<u64>,
    collection: Option<&crate::app::annotation_collection::chrome::Overlay<'_>>,
    _region: &State,
) {
    let group_drag = collection.is_some_and(|collection| collection.dragging);
    let dragged = req
        .dragged_index
        .zip(req.pending)
        .filter(|_| original_text_id.is_none());
    let annotations = if req.annotations_layer_empty && dragged.is_none() && !group_drag {
        &[][..]
    } else {
        req.annotations
    };
    let mut bodies: Vec<_> = annotations
        .iter()
        .filter(|annotation| Some(annotation.id) != original_text_id)
        .collect();
    let ordered_scene = !view.is_identity() || dragged.is_some() || group_drag;
    if let Some((index, pending)) = dragged.filter(|_| !group_drag) {
        bodies.insert(index.min(bodies.len()), pending);
    } else if !group_drag && ordered_scene {
        bodies.extend(req.pending);
    }
    let multiply = bodies.iter().copied().chain(req.pending).any(|annotation| {
        matches!(
            annotation.shape,
            crate::types::AnnotationShape::Highlighter { .. }
        )
    });
    let merged_focus = req
        .pending
        .is_some_and(crate::features::filter::focus::enabled)
        && bodies
            .iter()
            .copied()
            .any(crate::features::filter::focus::enabled);
    if (multiply || merged_focus) && !ordered_scene {
        bodies.extend(req.pending);
    }
    ordered(req, view, &bodies, multiply);
    if !ordered_scene && !multiply && !merged_focus {
        pending(req);
    }
}

#[cfg(test)]
#[path = "annotation_tiles_tests.rs"]
mod tests;
