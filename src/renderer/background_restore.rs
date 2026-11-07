//! 【截图绘制】【区域背景】直接恢复损坏像素，圆角使用固定完整范围保留抗锯齿相位

use super::{Painter, RenderRequest, dimming, rect_bounds, scale_rect};
use tiny_skia::{IntRect, Pixmap, Rect};

/// 【截图绘制】【损坏像素】按canvas、dirty及scale返回受画布约束的整数像素范围，无有效范围时返回空值
pub(super) fn damage(canvas: &Pixmap, dirty: Option<&Rect>, scale: f32) -> Option<IntRect> {
    let (x, y, width, height) = if let Some(dirty) = dirty {
        rect_bounds(&scale_rect(dirty, scale)?, canvas.width(), canvas.height())?
    } else {
        (0, 0, canvas.width(), canvas.height())
    };
    IntRect::from_xywh(x as i32, y as i32, width, height)
}

/// 【截图绘制】【区域填充】将pixels写入canvas的area，不参与透明混合，无返回值
fn fill(canvas: &mut Pixmap, area: IntRect, pixels: [u8; 4]) {
    let stride = canvas.width() as usize * 4;
    for row in area.top()..area.bottom() {
        let offset = row as usize * stride + area.left() as usize * 4;
        for pixel in canvas.data_mut()[offset..offset + area.width() as usize * 4]
            .as_chunks_mut::<4>()
            .0
        {
            *pixel = pixels;
        }
    }
}

/// 【截图绘制】【直角背景】按req与area写入遮罩和透明选区，整数取整与旧缓存保持一致，无返回值
fn identity(req: &mut RenderRequest<'_>, area: IntRect) {
    let hole = req
        .selection
        .and_then(|selection| scale_rect(selection, req.scale))
        .and_then(|rect| rect_bounds(&rect, req.canvas.width(), req.canvas.height()))
        .and_then(|(x, y, width, height)| IntRect::from_xywh(x as i32, y as i32, width, height))
        .and_then(|hole| hole.intersect(&area));
    // 1. 【截图绘制】【透明恢复】损坏范围完全位于选区时不写入无用遮罩，完整画布直接归还透明页，保留原有零值
    if hole == Some(area) {
        if area == IntRect::from_xywh(0, 0, req.canvas.width(), req.canvas.height()).unwrap() {
            crate::utils::transparent_pixmap::clear(req.canvas);
        } else {
            fill(req.canvas, area, [0; 4]);
        }
    } else {
        fill(req.canvas, area, dimming::dim_pixel());
        if let Some(hole) = hole {
            fill(req.canvas, hole, [0; 4])
        }
    }
    corners(req, area);
}

/// 【截图绘制】【区域复制】把tile的全局像素位置position内的area复制到canvas，无返回值，不合成透明度
fn copy(canvas: &mut Pixmap, tile: &Pixmap, position: (i32, i32), area: IntRect) {
    let stride = canvas.width() as usize * 4;
    let source_stride = tile.width() as usize * 4;
    let bytes = area.width() as usize * 4;
    for row in area.top()..area.bottom() {
        let source =
            (row - position.1) as usize * source_stride + (area.left() - position.0) as usize * 4;
        let target = row as usize * stride + area.left() as usize * 4;
        canvas.data_mut()[target..target + bytes]
            .copy_from_slice(&tile.data()[source..source + bytes]);
    }
}

/// 【截图绘制】【圆角恢复】按req与area恢复相交圆角，每次只保存单个圆角的像素，无返回值
fn corners(req: &mut RenderRequest<'_>, area: IntRect) {
    let (Some(selection), Some(edges)) = (req.selection, req.selection_edges) else {
        return;
    };
    let radius = dimming::hole_radius(req.selection_radius)
        .min(selection.width() / 2.0)
        .min(selection.height() / 2.0);
    if radius <= 0.0 {
        return;
    }
    for (visible, x, y) in [
        (edges.top && edges.left, selection.left(), selection.top()),
        (
            edges.top && edges.right,
            selection.right() - radius,
            selection.top(),
        ),
        (
            edges.bottom && edges.right,
            selection.right() - radius,
            selection.bottom() - radius,
        ),
        (
            edges.bottom && edges.left,
            selection.left(),
            selection.bottom() - radius,
        ),
    ] {
        let Some(bounds) = Rect::from_xywh(x, y, radius, radius)
            .and_then(|rect| scale_rect(&rect, req.scale))
            .and_then(|rect| rect_bounds(&rect, req.canvas.width(), req.canvas.height()))
            .and_then(|(x, y, width, height)| IntRect::from_xywh(x as i32, y as i32, width, height))
            .filter(|rect| visible && rect.intersect(&area).is_some())
        else {
            continue;
        };
        let Some(mut tile) = Pixmap::new(bounds.width(), bounds.height()) else {
            continue;
        };
        let stride = req.canvas.width() as usize * 4;
        let bytes = bounds.width() as usize * 4;
        for row in 0..bounds.height() as usize {
            let source = (bounds.top() as usize + row) * stride + bounds.left() as usize * 4;
            tile.data_mut()[row * bytes..(row + 1) * bytes]
                .copy_from_slice(&req.canvas.data()[source..source + bytes]);
        }
        dimming::draw_hole_corners(
            &mut Painter::new(&mut tile).with_scale(req.scale).with_origin((
                bounds.left() as f32 / req.scale,
                bounds.top() as f32 / req.scale,
            )),
            selection,
            Some(edges),
            req.selection_radius,
        );
        copy(
            req.canvas,
            &tile,
            (bounds.left(), bounds.top()),
            bounds.intersect(&area).unwrap(),
        );
    }
}

/// 【画布导航】【视口遮罩】按painter、selection和size绘制选区外遮罩，无返回值，与旧导航缓存使用同一矩形顺序
pub(super) fn shade(painter: &mut Painter<'_>, selection: Option<&Rect>, size: (f32, f32)) {
    let color = crate::config::get().selection.dim;
    if let Some(selection) = selection {
        for (left, top, right, bottom) in [
            (0.0, 0.0, size.0, selection.top()),
            (0.0, selection.bottom(), size.0, size.1),
            (0.0, selection.top(), selection.left(), selection.bottom()),
            (
                selection.right(),
                selection.top(),
                size.0,
                selection.bottom(),
            ),
        ] {
            if let Some(rect) = Rect::from_ltrb(left, top, right, bottom) {
                painter.fill_rounded(rect, 0.0, color);
            }
        }
    } else if let Some(rect) = Rect::from_xywh(0.0, 0.0, size.0, size.1) {
        painter.fill_rounded(rect, 0.0, color);
    }
}

/// 【截图绘制】【背景恢复】按req损坏范围恢复恒等视图的遮罩和透明选区，无返回值
pub(super) fn draw(req: &mut RenderRequest<'_>) {
    let Some(area) = damage(req.canvas, req.dirty_rect, req.scale) else {
        return;
    };
    identity(req, area);
}

#[cfg(test)]
#[path = "background_restore_tests.rs"]
mod tests;
