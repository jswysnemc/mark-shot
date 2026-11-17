//! 【聚焦滤镜】【遮罩合成】支持多孔洞并集单次压暗，防止多个聚焦区域叠加降低亮度

use super::{FilterMode, RegionFilter, draw::pixel_bounds};
use crate::{
    renderer::Painter,
    types::{Annotation, AnnotationShape},
};
use tiny_skia::{
    BlendMode, Color, FillRule, Mask, Paint, Path, PathBuilder, Pixmap, Rect, Transform,
};

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

/// 【聚焦滤镜】【孔洞几何】按 filter 生成矩形或椭圆旋转后的孔洞矢量路径，无效尺寸返回空值
fn hole_path(filter: RegionFilter) -> Option<Path> {
    let rect = filter.rect()?;
    if filter.mode == FilterMode::FocusEllipse {
        PathBuilder::from_oval(rect).and_then(|path| {
            path.transform(Transform::from_rotate_at(
                filter.rotation.to_degrees(),
                filter.center().0,
                filter.center().1,
            ))
        })
    } else {
        filter.path(rect)
    }
}

/// 【聚焦滤镜】【默认边界】按 painter 绘图限制或冻结网格返回聚焦遮罩作用范围
fn fallback_bounds(painter: &Painter<'_>) -> Option<Rect> {
    painter.drawing_bounds().or_else(|| {
        painter
            .frozen_image()?
            .native_grid()
            .map(|grid| grid.bounds)
    })
}

/// 【聚焦滤镜】【区域绘制】按 painter、filter 与 opacity 合成周围遮罩，不采样原图，不绘制彩色描边，无返回值
pub fn draw(painter: &mut Painter<'_>, filter: RegionFilter, opacity: f32) {
    let Some(bounds) = fallback_bounds(painter) else {
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

/// 【聚焦滤镜】【多孔洞合并绘制】将 items 中全部聚焦标注合并为单层外围压暗与多孔洞镂空，避免亮度降低叠加
pub fn draw_group<'a>(painter: &mut Painter<'_>, items: impl IntoIterator<Item = &'a Annotation>) {
    let mut entries: Vec<(RegionFilter, f32, Option<Rect>)> = items
        .into_iter()
        .filter_map(|ann| match ann.shape {
            AnnotationShape::Redact { filter } if filter.mode.is_focus() => {
                Some((filter, ann.color.alpha(), ann.frozen_bounds))
            }
            _ => None,
        })
        .collect();
    if entries.is_empty() {
        return;
    }
    if entries.len() == 1 {
        let (filter, opacity, frozen) = entries.pop().unwrap();
        if let Some(bounds) = painter
            .drawing_bounds()
            .or(frozen)
            .or_else(|| fallback_bounds(painter))
        {
            draw_in(painter, filter, opacity, bounds);
        }
        return;
    }

    // 1. 聚合多聚焦区域的联合边界与单次最大压暗透明度，不重复累乘暗度
    let bounds = painter.drawing_bounds().or_else(|| {
        entries
            .iter()
            .filter_map(|(_, _, b)| *b)
            .reduce(|acc, item| acc.join(&item).unwrap_or(acc))
            .or_else(|| fallback_bounds(painter))
    });
    let Some(bounds) = bounds else {
        return;
    };
    let mask_alpha = entries
        .iter()
        .map(|(filter, opacity, _)| {
            (f32::from(alpha(filter.strength)) * opacity.clamp(0.0, 1.0)).round() as u8
        })
        .max()
        .unwrap_or(0);
    if mask_alpha == 0 {
        return;
    }
    let holes: Vec<(RegionFilter, Path)> = entries
        .into_iter()
        .filter_map(|(filter, _, _)| hole_path(filter).map(|path| (filter, path)))
        .collect();
    if holes.is_empty() {
        return;
    }

    let scale = painter.scale();
    let phase = painter.pixel_phase();
    let origin = painter.origin();
    let size = painter.size();
    let Some(viewport) = Rect::from_xywh(origin.0, origin.1, size.0, size.1) else {
        return;
    };
    if holes
        .iter()
        .any(|(filter, _)| inside(*filter, viewport, 2.0 / scale))
    {
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
    let color = Color::from_rgba8(0, 0, 0, mask_alpha);

    // 2. 按固定物理网格单次填充外围暗色并依次镂空全部相交聚焦孔洞
    for row in visible.top().div_euclid(TILE)..=(visible.bottom() - 1).div_euclid(TILE) {
        for column in visible.left().div_euclid(TILE)..=(visible.right() - 1).div_euclid(TILE) {
            let (x, y) = (column * TILE, row * TILE);
            let local = (
                ((f64::from(x) + phase.0) / f64::from(scale)) as f32,
                ((f64::from(y) + phase.1) / f64::from(scale)) as f32,
            );
            let tile_bounds =
                Rect::from_xywh(local.0, local.1, TILE as f32 / scale, TILE as f32 / scale)
                    .unwrap();
            if holes
                .iter()
                .any(|(filter, _)| inside(*filter, tile_bounds, 2.0 / scale))
            {
                continue;
            }
            let Some(mut tile) = Pixmap::new(TILE as u32, TILE as u32) else {
                return;
            };
            let transform = Transform::from_translate(-local.0, -local.1).post_scale(scale, scale);
            let mut paint = Paint::default();
            paint.set_color(color);
            tile.fill_rect(fill, &paint, transform, None);
            let punch = Paint {
                blend_mode: BlendMode::DestinationOut,
                anti_alias: true,
                ..Default::default()
            };
            for (_, path) in &holes {
                if tile_bounds.intersect(&path.bounds()).is_some() {
                    tile.fill_path(path, &punch, FillRule::Winding, transform, None);
                }
            }
            let destination = painter.to_px(local.0, local.1);
            painter.draw_pixmap(
                destination.0.round() as i32,
                destination.1.round() as i32,
                &tile,
            );
        }
    }
}

/// 【聚焦滤镜】【边界内合成】按 painter、filter、opacity 与 bounds 绘制单孔洞有界遮罩，无返回值
fn draw_in(painter: &mut Painter<'_>, filter: RegionFilter, opacity: f32, bounds: Rect) {
    let Some(hole) = hole_path(filter) else {
        return;
    };
    let scale = painter.scale();
    let phase = painter.pixel_phase();
    let origin = painter.origin();
    let size = painter.size();
    let Some(viewport) = Rect::from_xywh(origin.0, origin.1, size.0, size.1) else {
        return;
    };
    // 1. 大范围孔洞内部没有遮罩贡献，不分配像素块或重复栅格化
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
    // 2. 完整块锚定原始输出相位，局部缓冲不会改变孔洞边缘的抗锯齿
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
            // 3. 只合成当前可见块，孔洞内部保持透明且没有图像缓存
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
