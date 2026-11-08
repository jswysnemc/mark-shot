//! 【渲染】【固定分块】保持路径栅格原点，空心选框仅提交边缘

use tiny_skia::{Color, Path, Pixmap, Rect};

use super::Painter;

const TILE_PIXELS: i32 = 1024;

/// 【渲染】【路径分块】把 path、color 绘入 painter；完整边界决定像素原点，局部重绘保持相位，无返回值
pub fn fill(painter: &mut Painter<'_>, path: &Path, color: Color) {
    fill_tiles(painter, path, color, TILE_PIXELS, None);
}

/// 【渲染】【空心边框】按path和color绘制边框，interior为保证透明的内部范围；仅绘制边缘小块，返回值为空
pub(super) fn fill_border(
    painter: &mut Painter<'_>,
    path: &Path,
    color: Color,
    interior: Option<Rect>,
) {
    fill_tiles(painter, path, color, 128, interior);
}

/// 【渲染】【固定栅格】按tile_pixels限定临时图像，跳过interior完整覆盖的块，将path以color填入painter，无返回值
fn fill_tiles(
    painter: &mut Painter<'_>,
    path: &Path,
    color: Color,
    tile_pixels: i32,
    interior: Option<Rect>,
) {
    let scale = painter.scale();
    if !scale.is_finite() || scale <= 0.0 || color.alpha() <= 0.0 {
        return;
    }
    let Some(body) = path.bounds().outset(2.0 / scale, 2.0 / scale) else {
        return;
    };
    let Some(pixels) = Rect::from_ltrb(
        body.left() * scale,
        body.top() * scale,
        body.right() * scale,
        body.bottom() * scale,
    )
    .and_then(|body| body.round_out()) else {
        return;
    };
    let origin = painter.origin();
    let size = painter.size();
    let Some(visible) = Rect::from_xywh(origin.0, origin.1, size.0, size.1)
        .and_then(|canvas| canvas.intersect(&body))
    else {
        return;
    };
    let first_column =
        ((visible.left() * scale).floor() as i64 - pixels.x() as i64).max(0) / tile_pixels as i64;
    let first_row =
        ((visible.top() * scale).floor() as i64 - pixels.y() as i64).max(0) / tile_pixels as i64;
    let last_column = (((visible.right() * scale).ceil() as i64 - pixels.x() as i64).max(1) - 1)
        / tile_pixels as i64;
    let last_row = (((visible.bottom() * scale).ceil() as i64 - pixels.y() as i64).max(1) - 1)
        / tile_pixels as i64;
    // 1. 【渲染】【路径分块】只生成可见完整像素块，不按整条大曲线分配巨型缓冲
    for row in first_row..=last_row {
        for column in first_column..=last_column {
            let x = (pixels.x() as i64 + column * tile_pixels as i64) as i32;
            let y = (pixels.y() as i64 + row * tile_pixels as i64) as i32;
            let width = (pixels.right() - x).clamp(0, tile_pixels) as u32;
            let height = (pixels.bottom() - y).clamp(0, tile_pixels) as u32;
            if interior.is_some_and(|area| {
                x as f32 / scale >= area.left()
                    && y as f32 / scale >= area.top()
                    && (x as f32 + width as f32) / scale <= area.right()
                    && (y as f32 + height as f32) / scale <= area.bottom()
            }) {
                continue;
            }
            let local_origin = (x as f32 / scale, y as f32 / scale);
            let Some(mut temporary) = Pixmap::new(width, height) else {
                continue;
            };
            painter
                .sub(&mut temporary)
                .with_origin(local_origin)
                .fill_path(path, color);
            // 2. 【渲染】【路径分块】箭身和头部共用一个填充，半透明颜色不会在接缝重复覆盖
            let destination = painter.to_px(local_origin.0, local_origin.1);
            painter.draw_pixmap(
                destination.0.round() as i32,
                destination.1.round() as i32,
                &temporary,
            );
        }
    }
}
