//! 【序号】【正文绘制】复用冻结坐标和固定像素分块，统一预览、局部重绘与原图导出

use tiny_skia::{Color, PathBuilder, Pixmap, Rect, Stroke};

use super::NumberMark;
use super::window_geometry::Geometry;
use crate::renderer::{Painter, number_text};

const TILE_PIXELS: i32 = 1024;

/// 【序号】【分块绘制】将 mark 按 width、color 绘入 painter；单个正文缓冲最多四 MiB，块原点不随损坏区域变化，无返回值
pub fn draw(painter: &mut Painter<'_>, mark: NumberMark, width: f32, color: Color) {
    let scale = painter.scale();
    if !mark.valid() || !scale.is_finite() || scale <= 0.0 {
        return;
    }
    let geometry = Geometry::new(mark, width, painter.window_units());
    let label = number_text::prepare(painter, geometry);
    let pad = painter.window_units().canvas(2.0);
    let Some(body) = geometry
        .bounds()
        .map(|bounds| {
            label
                .as_ref()
                .and_then(|label| label.bounds(mark))
                .and_then(|label| bounds.join(&label))
                .unwrap_or(bounds)
        })
        .and_then(|body| body.outset(pad, pad))
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
    let origin = painter.origin();
    let size = painter.size();
    let Some(visible) = Rect::from_xywh(origin.0, origin.1, size.0, size.1)
        .and_then(|canvas| canvas.intersect(&body))
    else {
        return;
    };
    let first_column =
        (((visible.left() * scale).floor() as i32 - pixels.x()).max(0) / TILE_PIXELS) as u32;
    let first_row =
        (((visible.top() * scale).floor() as i32 - pixels.y()).max(0) / TILE_PIXELS) as u32;
    let last_column = (((visible.right() * scale).ceil() as i32 - pixels.x()).max(1) - 1) as u32
        / TILE_PIXELS as u32;
    let last_row = (((visible.bottom() * scale).ceil() as i32 - pixels.y()).max(1) - 1) as u32
        / TILE_PIXELS as u32;
    // 1. 【序号】【分块绘制】完整正文决定块索引，分数缩放和局部重绘保持相同光栅采样相位
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
                body_draw(&mut local, geometry, color);
                if let Some(label) = &label {
                    number_text::draw(&mut local, label, mark, Color::WHITE);
                }
            }
            // 2. 【序号】【分块绘制】整块只合成一次，临时缓冲在当前块结束后释放
            let destination = painter.to_px(local_origin.0, local_origin.1);
            painter.draw_pixmap(
                destination.0.round() as i32,
                destination.1.round() as i32,
                &temporary,
            );
        }
    }
}

/// 【序号】【圆框与尾部】按geometry与color绘入painter，白色边框遵循原版独立于填充透明度的规则，无返回值
fn body_draw(painter: &mut Painter<'_>, geometry: Geometry, color: Color) {
    let mark = geometry.mark;
    if let Some(points) = geometry.tail() {
        let mut path = PathBuilder::new();
        path.move_to(points[0].0, points[0].1);
        path.line_to(points[1].0, points[1].1);
        path.line_to(points[2].0, points[2].1);
        path.close();
        if let Some(path) = path.finish() {
            painter.fill_path(&path, color);
        }
    }
    let mut circle = PathBuilder::new();
    circle.push_circle(mark.bubble.0, mark.bubble.1, geometry.radius);
    if let Some(circle) = circle.finish() {
        painter.fill_path(&circle, color);
        painter.stroke_path(
            &circle,
            Color::WHITE,
            &Stroke {
                width: geometry.outline,
                ..Stroke::default()
            },
        );
    }
}
