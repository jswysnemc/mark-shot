//! 【聚焦滤镜】【遮罩合成】只压暗目标外侧，固定像素块保持局部重绘和导出一致

use super::{FilterMode, RegionFilter, draw::pixel_bounds};
use crate::{
    renderer::Painter,
    types::{Annotation, AnnotationShape},
};
use tiny_skia::{Color, FillRule, Mask, Paint, PathBuilder, Pixmap, Rect, Transform};

const TILE: i32 = 256;

/// 【聚焦滤镜】【标注资格】按 annotation 返回是否为矩形或椭圆聚焦，其他标注返回 false
pub fn enabled(annotation: &Annotation) -> bool {
    matches!(annotation.shape, AnnotationShape::Redact { filter } if filter.mode.is_focus())
}

/// 【聚焦滤镜】【交互损坏】按 annotation 与 selected 返回移动时的新旧孔洞范围，生命周期损坏仍使用整个绘图边界
pub fn interaction_bounds(annotation: &Annotation, selected: bool) -> Rect {
    if !enabled(annotation) {
        return annotation.damage_bbox(selected);
    }
    let body = annotation.bbox.outset(2.0, 2.0).unwrap_or(annotation.bbox);
    if selected && let AnnotationShape::Redact { filter } = annotation.shape {
        return super::drag::editing_bounds(filter)
            .and_then(|area| body.join(&area))
            .unwrap_or(body);
    }
    body
}

/// 【聚焦滤镜】【遮罩强度】按 strength 返回黑色遮罩透明度，强度沿用滤镜范围，无其他参数
pub fn alpha(strength: f32) -> u8 {
    let strength = super::clamped_strength(strength);
    (255.0 * strength / (strength + 10.0)).round() as u8
}

/// 【聚焦滤镜】【透明内部】按 filter、area 与 margin 判断整个矩形是否位于孔洞安全内部，凸形四角均在内部时返回 true
fn inside(filter: RegionFilter, area: Rect, margin: f32) -> bool {
    let Some(rect) = filter.rect().and_then(|rect| rect.inset(margin, margin)) else {
        return false;
    };
    let radius = (filter.radius * filter.style_scale).min(rect.width().min(rect.height()) / 2.0);
    super::geometry::corners(area).into_iter().all(|point| {
        let point = filter.to_local(point);
        if filter.mode == FilterMode::FocusEllipse {
            let center = filter.center();
            ((point.0 - center.0) / (rect.width() / 2.0)).powi(2)
                + ((point.1 - center.1) / (rect.height() / 2.0)).powi(2)
                < 1.0
        } else if point.0 < rect.left()
            || point.0 > rect.right()
            || point.1 < rect.top()
            || point.1 > rect.bottom()
        {
            false
        } else {
            let nearest = (
                point.0.clamp(rect.left() + radius, rect.right() - radius),
                point.1.clamp(rect.top() + radius, rect.bottom() - radius),
            );
            (point.0 - nearest.0).hypot(point.1 - nearest.1) <= radius
        }
    })
}

/// 【聚焦滤镜】【无贡献块】按 annotation、area 与 margin 判断整块是否在透明孔洞内部，返回是否可以跳过合成
pub(crate) fn inside_hole(annotation: &Annotation, area: Rect, margin: f32) -> bool {
    matches!(annotation.shape, AnnotationShape::Redact { filter } if filter.mode.is_focus() && inside(filter, area, margin))
}

/// 【聚焦滤镜】【区域绘制】按 painter、filter 与 opacity 合成周围遮罩，不采样原图，不绘制彩色描边，无返回值
pub fn draw(painter: &mut Painter<'_>, filter: RegionFilter, opacity: f32) {
    let Some(bounds) = painter.drawing_bounds().or_else(|| {
        painter
            .frozen_image()?
            .native_grid()
            .map(|grid| grid.bounds)
    }) else {
        return;
    };
    draw_in(painter, filter, opacity, bounds);
}

/// 【聚焦滤镜】【标注绘制】按 painter 与 annotation 使用冻结绘图边界，当前绘图限制优先，返回空值
pub fn draw_annotation(painter: &mut Painter<'_>, annotation: &Annotation) {
    let AnnotationShape::Redact { filter } = annotation.shape else {
        return;
    };
    if let Some(bounds) = painter.drawing_bounds().or(annotation.frozen_bounds) {
        draw_in(painter, filter, annotation.color.alpha(), bounds);
    } else {
        draw(painter, filter, annotation.color.alpha());
    }
}

/// 【聚焦滤镜】【边界内合成】按 painter、filter、opacity 与 bounds 绘制有界遮罩，孔洞旋转到边界外时不扩大效果，无返回值
fn draw_in(painter: &mut Painter<'_>, filter: RegionFilter, opacity: f32, bounds: Rect) {
    let Some(rect) = filter.rect() else { return };
    let hole = if filter.mode == FilterMode::FocusEllipse {
        PathBuilder::from_oval(rect).and_then(|path| {
            path.transform(Transform::from_rotate_at(
                filter.rotation.to_degrees(),
                filter.center().0,
                filter.center().1,
            ))
        })
    } else {
        filter.path(rect)
    };
    let Some(hole) = hole else { return };
    let scale = painter.scale();
    let phase = painter.pixel_phase();
    let origin = painter.origin();
    let size = painter.size();
    let Some(viewport) = Rect::from_xywh(origin.0, origin.1, size.0, size.1) else {
        return;
    };
    // 1. 【聚焦滤镜】【透明跳过】大范围孔洞内部没有遮罩贡献，不分配像素块或重复栅格化
    if inside(filter, viewport, 2.0 / scale) {
        return;
    }
    let Some(visible) = viewport
        .intersect(&bounds)
        .and_then(|rect| pixel_bounds(rect, scale, phase))
    else {
        return;
    };
    let Some(fill) = bounds.intersect(&viewport) else {
        return;
    };
    let color = Color::from_rgba8(
        0,
        0,
        0,
        (f32::from(alpha(filter.strength)) * opacity.clamp(0.0, 1.0)).round() as u8,
    );
    // 2. 【聚焦滤镜】【固定网格】完整块锚定原始输出相位，局部缓冲不会改变孔洞边缘的抗锯齿
    for row in visible.top().div_euclid(TILE)..=(visible.bottom() - 1).div_euclid(TILE) {
        for column in visible.left().div_euclid(TILE)..=(visible.right() - 1).div_euclid(TILE) {
            let (x, y) = (column * TILE, row * TILE);
            let Some(mut tile) = Pixmap::new(TILE as u32, TILE as u32) else {
                return;
            };
            let local = (
                ((f64::from(x) + phase.0) / f64::from(scale)) as f32,
                ((f64::from(y) + phase.1) / f64::from(scale)) as f32,
            );
            let tile_bounds =
                Rect::from_xywh(local.0, local.1, TILE as f32 / scale, TILE as f32 / scale)
                    .unwrap();
            if inside(filter, tile_bounds, 2.0 / scale) {
                continue;
            }
            let transform = Transform::from_translate(-local.0, -local.1).post_scale(scale, scale);
            let mut paint = Paint::default();
            paint.set_color(color);
            if tile_bounds.intersect(&hole.bounds()).is_none() {
                tile.fill_rect(fill, &paint, transform, None);
            } else {
                let Some(mut mask) = Mask::new(TILE as u32, TILE as u32) else {
                    return;
                };
                mask.fill_path(&hole, FillRule::Winding, true, transform);
                mask.invert();
                tile.fill_rect(fill, &paint, transform, Some(&mask));
            }
            // 3. 【聚焦滤镜】【单次交付】只合成当前可见块，孔洞内部保持透明且没有图像缓存
            let destination = painter.to_px(local.0, local.1);
            painter.draw_pixmap(
                destination.0.round() as i32,
                destination.1.round() as i32,
                &tile,
            );
        }
    }
}

#[cfg(test)]
mod performance;
#[cfg(test)]
mod tests;
