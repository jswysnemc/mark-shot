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
    // 1. 【截图绘制】【透明恢复】交互帧复用已分配像素，避免整帧回收后在提交时重新缺页分配
    if hole == Some(area) {
        if area == IntRect::from_xywh(0, 0, req.canvas.width(), req.canvas.height()).unwrap() {
            req.canvas.data_mut().fill(0);
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
    shade_region(painter, selection, size, None);
}

/// 【画布导航】【区域遮罩】按 painter、selection、size 和 clip 只合成损坏范围，原矩形抗锯齿相位保持不变，无返回值
pub(super) fn shade_region(
    painter: &mut Painter<'_>,
    selection: Option<&Rect>,
    size: (f32, f32),
    clip: Option<IntRect>,
) {
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
                shade_rectangle(painter, rect, color, clip);
            }
        }
    } else if let Some(rect) = Rect::from_xywh(0.0, 0.0, size.0, size.1) {
        shade_rectangle(painter, rect, color, clip);
    }
}

/// 【画布导航】【遮罩矩形】将 rect 以 color 绘入 painter；整数内部快速填充，边缘保留原路径抗锯齿，无返回值
fn shade_rectangle(
    painter: &mut Painter<'_>,
    rect: Rect,
    color: crate::theme::Rgba,
    clip: Option<IntRect>,
) {
    let scale = painter.scale();
    let Some(pixels) = rect.transform(painter.transform()) else {
        return;
    };
    let canvas = painter.canvas();
    let full = IntRect::from_xywh(0, 0, canvas.width(), canvas.height()).unwrap();
    let Some(bounds) = clip.unwrap_or(full).intersect(&full) else {
        return;
    };
    let Some(outer) = pixels.round_out().and_then(|rect| rect.intersect(&bounds)) else {
        return;
    };
    let inner = IntRect::from_ltrb(
        pixels.left().ceil() as i32,
        pixels.top().ceil() as i32,
        pixels.right().floor() as i32,
        pixels.bottom().floor() as i32,
    )
    .and_then(|rect| rect.intersect(&bounds));
    let mut bands = Vec::with_capacity(4);
    if let Some(inner) = inner {
        let mut paint = tiny_skia::Paint::default();
        paint.set_color(color.color());
        paint.anti_alias = false;
        let width = canvas.width();
        let stride = width as usize * 4;
        let first = inner.top() as usize * stride;
        let last = inner.bottom() as usize * stride;
        super::parallel_rows::for_each(
            &mut canvas.data_mut()[first..last],
            stride,
            64,
            |_, bytes| {
                let height = (bytes.len() / stride) as u32;
                let mut band = tiny_skia::PixmapMut::from_bytes(bytes, width, height).unwrap();
                band.fill_rect(
                    Rect::from_xywh(
                        inner.left() as f32,
                        0.0,
                        inner.width() as f32,
                        height as f32,
                    )
                    .unwrap(),
                    &paint,
                    tiny_skia::Transform::identity(),
                    None,
                );
            },
        );
        bands.extend(
            [
                (outer.left(), outer.top(), outer.right(), inner.top()),
                (outer.left(), inner.bottom(), outer.right(), outer.bottom()),
                (outer.left(), inner.top(), inner.left(), inner.bottom()),
                (inner.right(), inner.top(), outer.right(), inner.bottom()),
            ]
            .into_iter()
            .filter_map(|(left, top, right, bottom)| IntRect::from_ltrb(left, top, right, bottom)),
        );
    } else {
        bands.push(outer);
    }
    // 1. 【画布导航】【边缘保真】最多四条单像素边，裁切沿整数物理网格，半透明边角不改用另一种覆盖算法
    for band in bands {
        let logical = Rect::from_xywh(
            band.x() as f32 / scale,
            band.y() as f32 / scale,
            band.width() as f32 / scale,
            band.height() as f32 / scale,
        )
        .unwrap();
        painter.clipped(Some(logical), |painter| {
            painter.fill_rounded(rect, 0.0, color)
        });
    }
}

/// 【截图绘制】【背景恢复】按req损坏范围恢复恒等视图的遮罩和透明选区，无返回值
pub(super) fn draw(req: &mut RenderRequest<'_>) {
    let Some(area) = damage(req.canvas, req.dirty_rect, req.scale) else {
        return;
    };
    identity(req, area);
}

/// 【截图场景】【透明恢复】按 req 清除损坏前景中原有非零内容，不触碰已经透明的整块像素，遮罩交由合成器绘制
pub(super) fn clear_foreground(req: &mut RenderRequest<'_>) {
    let Some(area) = damage(req.canvas, req.dirty_rect, req.scale) else {
        return;
    };
    let stride = req.canvas.width() as usize * 4;
    for row in area.top()..area.bottom() {
        let first = row as usize * stride + area.left() as usize * 4;
        let bytes = &mut req.canvas.data_mut()[first..first + area.width() as usize * 4];
        for block in bytes.chunks_mut(256) {
            if block
                .as_chunks::<4>()
                .0
                .iter()
                .any(|pixel| *pixel != [0; 4])
            {
                block.fill(0);
            }
        }
    }
    corners(req, area);
}

#[cfg(test)]
#[path = "background_restore_tests.rs"]
mod tests;
