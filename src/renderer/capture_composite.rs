//! 【截图】【像素合成】按逻辑区域与显示器比例合成原始截图

use super::*;

/// the highest scale among monitors the region touches
pub fn capture_scale(
    captures: &[crate::types::Capture],
    placements: &[crate::types::Placement],
    region: Rect,
) -> f32 {
    let max = placements
        .iter()
        .zip(captures)
        .filter(|(p, _)| {
            Rect::from_xywh(
                p.position.0 as f32,
                p.position.1 as f32,
                p.size.0 as f32,
                p.size.1 as f32,
            )
            .is_some_and(|r| crate::utils::rects_overlap(&r, &region))
        })
        .map(|(_, c)| c.scale)
        .fold(0.0_f32, f32::max);
    if max > 0.0 { max } else { 1.0 }
}

pub fn composite(
    captures: &[crate::types::Capture],
    placements: &[crate::types::Placement],
    region: Rect,
    scale: Option<f32>,
) -> Option<(Pixmap, (i32, i32), f32)> {
    let left = region.left().floor() as i32;
    let top = region.top().floor() as i32;
    let width = (region.right().ceil() as i32 - left).max(0) as u32;
    let height = (region.bottom().ceil() as i32 - top).max(0) as u32;
    if width == 0 || height == 0 {
        return None;
    }
    let area = Rect::from_xywh(left as f32, top as f32, width as f32, height as f32)?;

    let scale = scale.unwrap_or_else(|| capture_scale(captures, placements, area));

    let out_w = (width as f32 * scale).round() as u32;
    let out_h = (height as f32 * scale).round() as u32;
    let out = compose(
        captures,
        placements,
        (f64::from(left), f64::from(top)),
        (out_w, out_h),
        scale,
    )?;
    Some((out, (left, top), scale))
}

/// 【截图裁切】【物理合成】按 captures、placements、region 和可选 scale 返回物理像素、精确全局原点及比例
pub(crate) fn composite_precise(
    captures: &[crate::types::Capture],
    placements: &[crate::types::Placement],
    region: Rect,
    scale: Option<f32>,
) -> Option<(Pixmap, (f64, f64), f32)> {
    let workspace = crate::utils::get_full_workspace_rect(placements)?;
    let region = region.intersect(&workspace)?;
    let scale = scale.unwrap_or_else(|| capture_scale(captures, placements, region));
    let bounds = super::pixel_bounds::map(
        region,
        (workspace.left(), workspace.top()),
        f64::from(scale),
    )?;
    let origin = (
        f64::from(workspace.left()) + f64::from(bounds.x()) / f64::from(scale),
        f64::from(workspace.top()) + f64::from(bounds.y()) / f64::from(scale),
    );
    let out = compose(
        captures,
        placements,
        origin,
        (bounds.width(), bounds.height()),
        scale,
    )?;
    Some((out, origin, scale))
}

/// 【截图裁切】【像素写入】按 origin、size 和 scale 把 captures 合成到统一物理画布，placements 提供全局位置，返回缓冲
fn compose(
    captures: &[crate::types::Capture],
    placements: &[crate::types::Placement],
    origin: (f64, f64),
    size: (u32, u32),
    scale: f32,
) -> Option<Pixmap> {
    let mut out = Pixmap::new(size.0, size.1)?;

    for (placement, capture) in placements.iter().zip(captures) {
        let src = &capture.pixmap;
        // 1. 【截图裁切】【原生比例】使用捕获比例统一映射两轴，取整后的输出逻辑尺寸不能再次缩放原始缓冲
        let factor = scale / capture.scale;
        let tx = ((f64::from(placement.position.0) - origin.0) * f64::from(scale)) as f32;
        let ty = ((f64::from(placement.position.1) - origin.1) * f64::from(scale)) as f32;

        if factor == 1.0 && tx.fract() == 0.0 && ty.fract() == 0.0 {
            out.draw_pixmap(
                tx as i32,
                ty as i32,
                src.as_ref(),
                &tiny_skia::PixmapPaint::default(),
                Transform::identity(),
                None,
            );
        } else {
            out.draw_pixmap(
                0,
                0,
                src.as_ref(),
                &tiny_skia::PixmapPaint {
                    quality: tiny_skia::FilterQuality::Bicubic,
                    ..tiny_skia::PixmapPaint::default()
                },
                Transform::from_row(factor, 0.0, 0.0, factor, tx, ty),
                None,
            );
        }
    }

    Some(out)
}

#[cfg(test)]
mod tests;
