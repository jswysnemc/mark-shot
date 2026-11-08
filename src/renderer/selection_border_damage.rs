//! 【截图选框】【边缘裁切】先求边缘损坏交集，避免为透明内部复制整幅画布

use super::{Painter, dimming, scale_rect};
use crate::types::SelectionEdges;
use tiny_skia::{IntRect, Pixmap, Rect};

/// 【截图选框】【有效边缘】按selection、scale和radius返回不重叠的物理边缘矩形，保留圆角与抗锯齿范围
fn bands(selection: &Rect, scale: f32, radius: f32) -> Vec<IntRect> {
    let padding = crate::config::get().selection.border_width * 2.0 + 2.0 / scale;
    let Some(outer) = selection
        .outset(padding, padding)
        .and_then(|area| scale_rect(&area, scale))
        .and_then(|area| area.round_out())
    else {
        return Vec::new();
    };
    let inner = selection
        .inset(radius + padding, radius + padding)
        .and_then(|area| scale_rect(&area, scale))
        .and_then(|area| {
            IntRect::from_ltrb(
                area.left().ceil() as i32,
                area.top().ceil() as i32,
                area.right().floor() as i32,
                area.bottom().floor() as i32,
            )
        });
    let Some(inner) = inner else {
        return vec![outer];
    };
    [
        (outer.left(), outer.top(), outer.right(), inner.top()),
        (outer.left(), inner.bottom(), outer.right(), outer.bottom()),
        (outer.left(), inner.top(), inner.left(), inner.bottom()),
        (inner.right(), inner.top(), outer.right(), inner.bottom()),
    ]
    .into_iter()
    .filter_map(|(left, top, right, bottom)| IntRect::from_ltrb(left, top, right, bottom))
    .collect()
}

/// 【截图选框】【损坏绘制】按scale将selection和edges绘入canvas，仅处理dirty内的边缘；radius决定圆角，无返回值
pub(super) fn draw(
    canvas: &mut Pixmap,
    scale: f32,
    dirty: Option<Rect>,
    selection: &Rect,
    edges: Option<&SelectionEdges>,
    radius: f32,
) {
    if !scale.is_finite() || scale <= 0.0 || edges.is_none() {
        return;
    }
    let bounds = IntRect::from_xywh(0, 0, canvas.width(), canvas.height()).unwrap();
    let damage = match dirty {
        Some(dirty) => scale_rect(&dirty, scale)
            .and_then(|area| area.round_out())
            .and_then(|area| area.intersect(&bounds)),
        None => Some(bounds),
    };
    let Some(damage) = damage else { return };
    // 1. 【截图选框】【像素交集】按整数像素分离四边，分数比例下也不会重复混合接缝
    for band in bands(selection, scale, radius) {
        let Some(area) = band.intersect(&damage) else {
            continue;
        };
        let Some(mut local) = canvas.clone_rect(area) else {
            continue;
        };
        dimming::draw_selection_border(
            &mut Painter::new(&mut local)
                .with_scale(scale)
                .with_origin((area.left() as f32 / scale, area.top() as f32 / scale)),
            selection,
            edges,
            radius,
        );
        // 2. 【截图选框】【范围替换】直接覆盖已保留背景的边缘，不再次混合透明颜色
        let stride = canvas.width() as usize * 4;
        let local_stride = area.width() as usize * 4;
        for (row, source) in local.data().chunks_exact(local_stride).enumerate() {
            let first = (area.top() as usize + row) * stride + area.left() as usize * 4;
            canvas.data_mut()[first..first + local_stride].copy_from_slice(source);
        }
    }
}
