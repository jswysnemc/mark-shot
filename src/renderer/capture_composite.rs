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
    let mut out = Pixmap::new(out_w, out_h)?;

    for (placement, capture) in placements.iter().zip(captures) {
        let src = &capture.pixmap;
        let sx = scale * placement.size.0 as f32 / src.width() as f32;
        let sy = scale * placement.size.1 as f32 / src.height() as f32;
        let tx = (placement.position.0 - left) as f32 * scale;
        let ty = (placement.position.1 - top) as f32 * scale;

        if sx == 1.0 && sy == 1.0 && tx.fract() == 0.0 && ty.fract() == 0.0 {
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
                Transform::from_row(sx, 0.0, 0.0, sy, tx, ty),
                None,
            );
        }
    }

    Some((out, (left, top), scale))
}
