//! 【尺子草稿】【单层分块】复用尺子自身的固定栅格，避免每个外层块重新生成相同正文与刻度

use super::{Painter, RenderRequest, annotations};
use crate::types::Annotation;

/// 【尺子草稿】【绘制入口】按req的损坏范围将pending直接叠加到已恢复前景，无返回值；正文内部仍使用固定像素原点
pub(super) fn draw(req: &mut RenderRequest<'_>, pending: &Annotation) {
    let bounds = pending.damage_bbox(false);
    let Some(bounds) = tiny_skia::Rect::from_xywh(
        bounds.left() - req.offset.0,
        bounds.top() - req.offset.1,
        bounds.width(),
        bounds.height(),
    )
    .and_then(|area| area.outset(2.0 / req.scale, 2.0 / req.scale)) else {
        return;
    };
    let area = match req.dirty_rect {
        Some(dirty) => bounds.intersect(dirty),
        None => Some(bounds),
    };
    let Some(area) = area else { return };
    let mut painter = Painter::new(req.canvas)
        .with_scale(req.scale)
        .with_origin(req.offset)
        .with_frozen_image(req.frozen_image)
        .with_drawing_bounds(req.drawing_bounds)
        .with_text(req.text.font_system, req.text.swash_cache);
    painter.clipped(Some(area), |painter| {
        annotations::draw_annotation(
            painter,
            pending,
            false,
            req.text.editors,
            req.filters,
            req.active_text_id,
        );
    });
}
