//! 【多选区截图】【像素交付】多显示器共用同一物理网格，未选中的像素不进入 PNG

use super::geometry;
use crate::types::{Capture, Placement};
use tiny_skia::{FillRule, Mask, PathBuilder, Pixmap, Rect, Transform};

/// 【多选区截图】【透明裁切】按 regions 的并集合成 captures，返回 PNG、精确桌面原点与像素倍率；空区域返回空值
pub(crate) fn render(
    captures: &[Capture],
    placements: &[Placement],
    regions: &[Rect],
) -> Option<(Vec<u8>, (f64, f64), f32)> {
    let (pixels, origin, scale) = compose(captures, placements, regions)?;
    Some((crate::utils::encode_png_with_alpha(pixels), origin, scale))
}

/// 【多选区截图】【编辑原图】按 regions 裁切 captures，返回已清除未选像素的原图、桌面原点与倍率，编辑与直接导出共用此结果
pub(crate) fn compose(
    captures: &[Capture],
    placements: &[Placement],
    regions: &[Rect],
) -> Option<(Pixmap, (f64, f64), f32)> {
    let bounds = geometry::bounds(regions)?;
    let (mut pixels, origin, scale) =
        crate::renderer::composite_precise(captures, placements, bounds, None)?;
    // 1. 【多选区截图】【坐标投影】使用合成器返回的实际原点，不把分数倍率或负桌面坐标提前取整
    let mut path = PathBuilder::new();
    for rect in regions {
        let edge = |value: f32, base: f64| ((f64::from(value) - base) * f64::from(scale)) as f32;
        path.push_rect(Rect::from_ltrb(
            edge(rect.left(), origin.0),
            edge(rect.top(), origin.1),
            edge(rect.right(), origin.0),
            edge(rect.bottom(), origin.1),
        )?);
    }
    // 2. 【多选区截图】【遮罩交付】在编码前清除空隙的全部颜色通道，不把完整外接框当作截图
    let mut mask = Mask::new(pixels.width(), pixels.height())?;
    mask.fill_path(
        &path.finish()?,
        FillRule::Winding,
        false,
        Transform::identity(),
    );
    pixels.apply_mask(&mask);
    Some((pixels, origin, scale))
}
