//! 【界面】【背景遮罩】局部更新选区外的遮罩和图像层

use super::*;

#[cfg(test)]
#[path = "selection_border_tests.rs"]
mod tests;

/// 【截图选框】【遮罩半径】按 radius 和配置描边宽度返回透明区域半径，零半径保留直角
pub(super) fn hole_radius(radius: f32) -> f32 {
    let sel = &crate::config::get().selection;
    radius - sel.border_width / 2.0
}

pub(super) fn dim_pixel() -> [u8; 4] {
    let crate::theme::Rgba(r, g, b, a) = crate::config::get().selection.dim;
    let px = tiny_skia::ColorU8::from_rgba(r, g, b, a).premultiply();
    [px.red(), px.green(), px.blue(), px.alpha()]
}

pub fn init_dimming(
    dimmed: &mut Pixmap,
    selection: Option<&Rect>,
    edges: Option<&SelectionEdges>,
    scale: f32,
) {
    let px = dim_pixel();
    for d in dimmed.data_mut().as_chunks_mut::<4>().0 {
        d.copy_from_slice(&px);
    }

    if let Some(sel) = selection {
        fill_rect(dimmed, sel, [0; 4], scale);
        dim_hole_corners(dimmed, sel, edges, scale, 0.0);
    }
}

/// 【截图选框】【边框绘制】按 radius 把 sel 和可见 edges 绘入 p，局部损坏不改变描边栅格，无返回值
pub(super) fn draw_selection_border(
    p: &mut Painter,
    sel: &Rect,
    edges: Option<&SelectionEdges>,
    radius: f32,
) {
    let cfg = &crate::config::get().selection;
    let origin = p.origin();
    let size = p.size();
    let inset = radius + cfg.border_width * 2.0 + 2.0 / p.scale();
    // 1. 【截图选框】【内部损坏】尺子和画笔只改变选区内部时跳过外围描边，不创建固定路径临时缓冲
    if origin.0 >= sel.left() + inset
        && origin.1 >= sel.top() + inset
        && origin.0 + size.0 <= sel.right() - inset
        && origin.1 + size.1 <= sel.bottom() - inset
    {
        return;
    }
    let stroke = Stroke {
        width: cfg.border_width,
        ..Stroke::default()
    };

    if let Some(edges) = edges {
        let half = stroke.width / 2.0;
        let outer = Rect::from_ltrb(
            sel.left() - half,
            sel.top() - half,
            sel.right() + half,
            sel.bottom() + half,
        )
        .unwrap_or(*sel);

        if let Some(path) = rounded_rect_path(
            &outer,
            radius,
            edges.top && edges.left,
            edges.top && edges.right,
            edges.bottom && edges.right,
            edges.bottom && edges.left,
        ) {
            // 2. 【截图选框】【固定栅格】先生成完整描边，再按固定像素原点填充，局部裁切不会重新采样圆角
            if let Some(outline) = path.stroke(&stroke, p.scale()) {
                super::fixed_path::fill_border(
                    p,
                    &outline,
                    cfg.border.get().color(),
                    sel.inset(inset, inset),
                );
            }
        }
    }
}

/// Darken the four corners between the sharp rectangle and the rounded border.
/// Each corner is very small (at most `hole_radius()` square), so this is fast
/// and uses almost no performance.
///
/// A corner is only rounded if both sides are visible screen edges. If the
/// selection goes off the edge of the monitor, that corner stays flat.
pub(super) fn dim_hole_corners(
    canvas: &mut Pixmap,
    sel: &Rect,
    edges: Option<&SelectionEdges>,
    scale: f32,
    radius: f32,
) {
    draw_hole_corners(
        &mut Painter::new(canvas).with_scale(scale),
        sel,
        edges,
        radius,
    );
}

/// 【截图选框】【圆角遮罩】按p、sel、edges与radius绘制圆角内侧遮罩，继承局部原点，无返回值
pub(super) fn draw_hole_corners(
    p: &mut Painter<'_>,
    sel: &Rect,
    edges: Option<&SelectionEdges>,
    radius: f32,
) {
    let Some(edges) = edges else { return };
    let radius = hole_radius(radius)
        .min(sel.width() / 2.0)
        .min(sel.height() / 2.0);
    if radius <= 0.0 {
        return;
    }

    let dim = crate::config::get().selection.dim;

    // corner point, then the direction the rectangle's interior lies in
    let corners = [
        (edges.top && edges.left, (sel.left(), sel.top()), (1.0, 1.0)),
        (
            edges.top && edges.right,
            (sel.right(), sel.top()),
            (-1.0, 1.0),
        ),
        (
            edges.bottom && edges.right,
            (sel.right(), sel.bottom()),
            (-1.0, -1.0),
        ),
        (
            edges.bottom && edges.left,
            (sel.left(), sel.bottom()),
            (1.0, -1.0),
        ),
    ];

    for (rounded, (cx, cy), (sx, sy)) in corners {
        if !rounded {
            continue;
        }
        let mut pb = PathBuilder::new();
        pb.move_to(cx, cy);
        pb.line_to(cx + sx * radius, cy);
        pb.cubic_to(
            cx + sx * radius * (1.0 - paths::KAPPA),
            cy,
            cx,
            cy + sy * radius * (1.0 - paths::KAPPA),
            cx,
            cy + sy * radius,
        );
        pb.close();
        if let Some(path) = pb.finish() {
            p.fill_path(&path, dim);
        }
    }
}

pub(super) fn update_dimming_delta(
    dimmed: &mut Pixmap,
    prev: Option<&Rect>,
    next: Option<&Rect>,
    edges: Option<&SelectionEdges>,
    scale: f32,
    radius: f32,
) {
    if let Some(old) = prev {
        fill_rect(dimmed, old, dim_pixel(), scale);
    }
    if let Some(cur) = next {
        fill_rect(dimmed, cur, [0; 4], scale);
        dim_hole_corners(dimmed, cur, edges, scale, radius);
    }
}

pub(super) fn fill_rect(pixmap: &mut Pixmap, rect: &Rect, px: [u8; 4], scale: f32) {
    let (w, h) = (pixmap.width(), pixmap.height());
    let Some((x, y, rw, rh)) = scale_rect(rect, scale).and_then(|r| rect_bounds(&r, w, h)) else {
        return;
    };

    let stride = (w * 4) as usize;
    let row_bytes = rw as usize * 4;
    let data = pixmap.data_mut();
    for row in 0..rh {
        let off = (y + row) as usize * stride + x as usize * 4;
        for d in data[off..off + row_bytes].as_chunks_mut::<4>().0 {
            d.copy_from_slice(&px);
        }
    }
}

pub(super) fn blit_rect(src: &Pixmap, dst: &mut Pixmap, rect: &Rect, scale: f32) {
    let (w, h) = (dst.width(), dst.height());
    let Some((x, y, rw, rh)) = scale_rect(rect, scale).and_then(|r| rect_bounds(&r, w, h)) else {
        return;
    };

    let row_bytes = (rw * 4) as usize;
    let src_stride = (src.width() * 4) as usize;
    let dst_stride = (dst.width() * 4) as usize;

    let src_data = src.data();
    let dst_data = dst.data_mut();

    for row in 0..rh {
        let sy = (y + row) as usize;
        let sx = x as usize;
        let src_off = sy * src_stride + sx * 4;

        let dy = sy;
        let dx = sx;
        let dst_off = dy * dst_stride + dx * 4;

        dst_data[dst_off..dst_off + row_bytes]
            .copy_from_slice(&src_data[src_off..src_off + row_bytes]);
    }
}

pub(super) fn blit_annotations(src: &Pixmap, dst: &mut Pixmap, rect: &Rect, scale: f32) {
    let (w, h) = (dst.width(), dst.height());
    let Some((x, y, rw, rh)) = scale_rect(rect, scale).and_then(|r| rect_bounds(&r, w, h)) else {
        return;
    };
    let src_stride = (src.width() * 4) as usize;
    let dst_stride = (dst.width() * 4) as usize;
    let src_data = src.data();
    let dst_data = dst.data_mut();

    for row in 0..rh {
        let sy = (y + row) as usize;
        let sx = x as usize;
        let src_off = sy * src_stride + sx * 4;
        let dst_off = sy * dst_stride + sx * 4;

        for col in 0..rw as usize {
            let s = &src_data[src_off + col * 4..src_off + col * 4 + 4];
            let d = &mut dst_data[dst_off + col * 4..dst_off + col * 4 + 4];
            let sa = s[3] as u32;
            if sa == 0 {
                continue;
            }
            if sa == 255 {
                d.copy_from_slice(s);
                continue;
            }
            let inv = 255 - sa;
            d[0] = (s[0] as u32 + (d[0] as u32 * inv + 127) / 255).min(255) as u8;
            d[1] = (s[1] as u32 + (d[1] as u32 * inv + 127) / 255).min(255) as u8;
            d[2] = (s[2] as u32 + (d[2] as u32 * inv + 127) / 255).min(255) as u8;
            d[3] = (sa + (d[3] as u32 * inv + 127) / 255).min(255) as u8;
        }
    }
}

// since pixels are opaque, there is no need in blending
// and copying rows is much faster than draw_pixmap
