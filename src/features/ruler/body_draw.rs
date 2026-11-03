//! 【尺子】【固定像素绘制】按稳定原点分块绘制正文，整帧与局部重绘使用相同光栅采样

use tiny_skia::{Color, Pixmap, Rect};

use super::{Measurement, RulerGeometry};
use crate::renderer::Painter;

const TILE_PIXELS: i32 = 1024;

/// 【尺子】【正文合成】把 measurement 按 width、color 绘入 painter；单个临时块不超过 4 MiB，块坐标不随损坏区域变化，无返回值
pub fn draw(painter: &mut Painter<'_>, measurement: Measurement, width: f32, color: Color) {
    if color.alpha() <= 0.0 {
        return;
    }
    let scale = painter.scale();
    if !scale.is_finite() || scale <= 0.0 {
        return;
    }
    let Some(body) = measurement
        .visual_bounds(measurement.unit(width))
        .and_then(|body| body.outset(2.0 * measurement.style_scale, 2.0 * measurement.style_scale))
    else {
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
    let canvas = painter.size();
    let origin = painter.origin();
    let Some(visible) = Rect::from_xywh(origin.0, origin.1, canvas.0, canvas.1)
        .and_then(|canvas| canvas.intersect(&body))
    else {
        return;
    };
    if visible.is_empty() {
        return;
    }
    let first_column =
        (((visible.left() * scale).floor() as i32 - pixels.x()).max(0) / TILE_PIXELS) as u32;
    let first_row =
        (((visible.top() * scale).floor() as i32 - pixels.y()).max(0) / TILE_PIXELS) as u32;
    let last_column = (((visible.right() * scale).ceil() as i32 - pixels.x()).max(1) - 1) as u32
        / TILE_PIXELS as u32;
    let last_row = (((visible.bottom() * scale).ceil() as i32 - pixels.y()).max(1) - 1) as u32
        / TILE_PIXELS as u32;
    // 1. 【尺子】【正文合成】块索引来自完整正文边界，不随局部重绘原点或范围改变
    for row in first_row..=last_row {
        for column in first_column..=last_column {
            let x = pixels.x() + column as i32 * TILE_PIXELS;
            let y = pixels.y() + row as i32 * TILE_PIXELS;
            let width_pixels = (pixels.right() - x).clamp(0, TILE_PIXELS) as u32;
            let height_pixels = (pixels.bottom() - y).clamp(0, TILE_PIXELS) as u32;
            let Some(mut temporary) = Pixmap::new(width_pixels, height_pixels) else {
                continue;
            };
            let local_origin = (x as f32 / scale, y as f32 / scale);
            {
                let mut local = painter.sub(&mut temporary).with_origin(local_origin);
                match measurement.geometry {
                    RulerGeometry::Length { .. } => {
                        super::length_draw::draw(&mut local, measurement, width, color)
                    }
                    RulerGeometry::Angle { .. } => {
                        super::angle_draw::draw(&mut local, measurement, width, color)
                    }
                }
            }
            // 2. 【尺子】【正文合成】目标裁切由画布完成，块缓冲在本轮合成后立即释放
            let destination = painter.to_px(local_origin.0, local_origin.1);
            painter.draw_pixmap(
                destination.0.round() as i32,
                destination.1.round() as i32,
                &temporary,
            );
        }
    }
}
