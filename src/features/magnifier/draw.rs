//! 【放大镜】【像素绘制】按稳定像素原点绘制冻结原图、双框与连接线，不保存图像缓存

use tiny_skia::{
    Color, FillRule, LineCap, LineJoin, Mask, Paint, Path, PathBuilder, Pixmap, Rect, Stroke,
};

use super::{Magnifier, MagnifierShape};
use crate::renderer::Painter;

const TILE_PIXELS: i32 = 1024;

/// 【放大镜】【框路径】用 magnifier 的 shape、rotation 与 rect 创建完整路径；返回路径，空矩形返回空值
pub fn frame_path(magnifier: Magnifier, rect: Rect) -> Option<Path> {
    let mut path = PathBuilder::new();
    append_frame(&mut path, magnifier, rect);
    path.finish()
}

/// 【放大镜】【路径追加】把 rect 按 magnifier 的形状与旋转追加到 path；无返回值
fn append_frame(path: &mut PathBuilder, magnifier: Magnifier, rect: Rect) {
    if magnifier.shape == MagnifierShape::Circle {
        let center = magnifier.to_visual((
            (rect.left() + rect.right()) / 2.0,
            (rect.top() + rect.bottom()) / 2.0,
        ));
        path.push_circle(center.0, center.1, rect.width().min(rect.height()) / 2.0);
    } else {
        if let Some(rounded) = crate::features::shape::path(
            rect,
            magnifier.radius / magnifier.pixel_scale,
            magnifier.rotation,
            magnifier.pivot,
        ) {
            path.push_path(&rounded);
        }
    }
}

/// 【放大镜】【向内边框】以 width、color 绘制 magnifier 的 rect 边框；偶奇填充保持全部像素位于框内，无返回值
fn border(painter: &mut Painter<'_>, magnifier: Magnifier, rect: Rect, width: f32, color: Color) {
    if color.alpha() <= 0.0 {
        return;
    }
    let mut path = PathBuilder::new();
    append_frame(&mut path, magnifier, rect);
    if let Some(inner) = Rect::from_ltrb(
        rect.left() + width,
        rect.top() + width,
        rect.right() - width,
        rect.bottom() - width,
    )
    .filter(|rect| !rect.is_empty())
    {
        let outer_radius =
            (magnifier.radius / magnifier.pixel_scale).min(rect.width().min(rect.height()) / 2.0);
        let inner_frame = Magnifier {
            radius: (outer_radius - width).max(0.0) * magnifier.pixel_scale,
            ..magnifier
        };
        append_frame(&mut path, inner_frame, inner);
    }
    let Some(path) = path.finish() else { return };
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;
    let transform = painter.transform();
    painter
        .canvas()
        .fill_path(&path, &paint, FillRule::EvenOdd, transform, None);
}

/// 【放大镜】【透明乘法】返回 value 乘以 factor 的八位四舍五入结果，用于覆盖率与预乘合成
fn multiply(value: u8, factor: u8) -> u8 {
    ((u16::from(value) * u16::from(factor) + 127) / 255) as u8
}

/// 【放大镜】【原图放大】把 painter 冻结原图按 magnifier 映射到镜框；pixel_origin 为固定像素原点，保留裁切与透明度，无返回值
fn sample_lens(painter: &mut Painter<'_>, magnifier: Magnifier, pixel_origin: (i32, i32)) {
    let Some(image) = painter.frozen_image() else {
        return;
    };
    let (Some(lens), Some(source)) = (magnifier.lens_rect(), magnifier.source_rect()) else {
        return;
    };
    let Some(path) = frame_path(magnifier, lens) else {
        return;
    };
    let scale = painter.scale();
    let (width, height) = (painter.canvas().width(), painter.canvas().height());
    let Some(mut mask) = Mask::new(width, height) else {
        return;
    };
    mask.fill_path(&path, FillRule::Winding, true, painter.transform());
    let coverage = mask.data();
    let data = painter.canvas().data_mut();
    // 1. 【放大镜】【原图放大】使用固定全局像素中心反旋转取样，局部重绘不会改变缩放相位
    for row in 0..height {
        for column in 0..width {
            let index = row as usize * width as usize + column as usize;
            let mask = coverage[index];
            if mask == 0 {
                continue;
            }
            let point = (
                ((pixel_origin.0 + column as i32) as f32 + 0.5) / scale,
                ((pixel_origin.1 + row as i32) as f32 + 0.5) / scale,
            );
            let local = magnifier.to_local(point);
            let sample = (
                source.left() + (local.0 - lens.left()) * source.width() / lens.width(),
                source.top() + (local.1 - lens.top()) * source.height() / lens.height(),
            );
            let sample = (
                sample.0.clamp(source.left(), source.right().next_down()),
                sample.1.clamp(source.top(), source.bottom().next_down()),
            );
            let Some(pixel) = image.sample(sample) else {
                continue;
            };
            let alpha = multiply(pixel[3], mask);
            let offset = index * 4;
            // 2. 【放大镜】【原图放大】透明原图仍采用预乘覆盖合成，不把源框内已有标注复制进镜框
            for channel in 0..4 {
                data[offset + channel] = multiply(pixel[channel], mask)
                    .saturating_add(multiply(data[offset + channel], 255 - alpha));
            }
        }
    }
}

/// 【放大镜】【块正文】把 magnifier 的连接线、原图和边框绘入 painter；width、color 决定边框，pixel_origin 保持采样稳定，无返回值
fn draw_tile(
    painter: &mut Painter<'_>,
    magnifier: Magnifier,
    width: f32,
    color: Color,
    pixel_origin: (i32, i32),
) {
    let width = painter.window_units().stroke(width, 1.5, Some(18.0));
    if let Some(connectors) = magnifier.connectors() {
        let mut path = PathBuilder::new();
        for (start, end) in connectors {
            path.move_to(start.0, start.1);
            path.line_to(end.0, end.1);
        }
        if let Some(path) = path.finish() {
            painter.stroke_path(
                &path,
                color,
                &Stroke {
                    width,
                    line_cap: LineCap::Round,
                    line_join: LineJoin::Round,
                    ..Stroke::default()
                },
            );
        }
    }
    sample_lens(painter, magnifier, pixel_origin);
    for rect in [magnifier.lens_rect(), magnifier.source_rect()]
        .into_iter()
        .flatten()
    {
        border(painter, magnifier, rect, width, color);
    }
}

/// 【放大镜】【正文合成】把 magnifier 按 width、color 绘入 painter；按需临时像素块最多四 MiB，未旋转源框取样不包含标注，无返回值
pub fn draw(painter: &mut Painter<'_>, magnifier: Magnifier, width: f32, color: Color) {
    if !magnifier.measures() {
        return;
    }
    let scale = painter.scale();
    if !scale.is_finite() || scale <= 0.0 {
        return;
    }
    let window = painter.window_units();
    let padding = window.stroke(width, 1.5, Some(18.0)) / 2.0 + window.canvas(2.0);
    let Some(body) = magnifier
        .bounds()
        .and_then(|rect| rect.outset(padding, padding))
    else {
        return;
    };
    let Some(pixels) = Rect::from_ltrb(
        body.left() * scale,
        body.top() * scale,
        body.right() * scale,
        body.bottom() * scale,
    )
    .and_then(|rect| rect.round_out()) else {
        return;
    };
    let size = painter.size();
    let origin = painter.origin();
    let Some(visible) = Rect::from_xywh(origin.0, origin.1, size.0, size.1)
        .and_then(|rect| rect.intersect(&body))
        .filter(|rect| !rect.is_empty())
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
    // 1. 【放大镜】【正文合成】像素块原点来自完整双框，不随目标画布或损坏区域改变
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
            draw_tile(
                &mut painter.sub(&mut temporary).with_origin(local_origin),
                magnifier,
                width,
                color,
                (x, y),
            );
            let destination = painter.to_px(local_origin.0, local_origin.1);
            painter.draw_pixmap(
                destination.0.round() as i32,
                destination.1.round() as i32,
                &temporary,
            );
        }
    }
}
