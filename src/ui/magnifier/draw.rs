use crate::config::Magnifier;
use crate::renderer::Painter;
use crate::types::Capture;
use crate::ui::magnifier::{cells, offset, sample_pixel, size, zoom};
use tiny_skia::{Paint, PathBuilder, Pixmap, PixmapPaint, Rect, Stroke, Transform};

/// 【光标放大区】【样式读取】无参数，返回当前放大预览配置引用
fn cfg() -> &'static Magnifier {
    &crate::config::get().magnifier
}

/// 【光标放大区】【完整范围】按 cursor 和屏幕尺寸返回放大图、颜色与坐标的整体范围
pub fn magnifier_rect(cursor: (f32, f32), monitor_w: f32, monitor_h: f32) -> Rect {
    let height = box_height();
    let (mag_x, mag_y) = magnifier_position(cursor, (0.0, 0.0, monitor_w, monitor_h), height);
    Rect::from_xywh(mag_x, mag_y, super::readout::width(), height).unwrap()
}

/// 【光标放大区】【整体高度】无参数，返回圆镜与双行读数的整体高度
fn box_height() -> f32 {
    size() as f32 + cfg().label_gap + cfg().label_height * 2.0
}

/// 【光标放大区】【实时绘制】按 capture、cursor 和屏幕尺寸向 p 绘制局部像素、原图颜色与坐标，无返回值
pub fn draw_magnifier(
    p: &mut Painter,
    capture: &Capture,
    cursor: (f32, f32),
    (screen_w, screen_h): (f32, f32),
) {
    let scale = p.scale();
    let source = &capture.pixmap;
    let native = capture.to_native((cursor.0 as f64, cursor.1 as f64));

    let sample_size = cells() as i32;

    let half = (cells() / 2) as i32;
    let src_x = native.0.floor() as i32 - half;
    let src_y = native.1.floor() as i32 - half;

    // 1. 【光标放大区】【像素对齐】边缘保持指针像素居中，超出原图的部分使用底色填充
    let mut cropped = Pixmap::new(sample_size as u32, sample_size as u32).unwrap();
    cropped.fill(cfg().label_background.get().color());
    cropped.draw_pixmap(
        -src_x,
        -src_y,
        source.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        None,
    );

    let bounds = magnifier_rect(cursor, screen_w, screen_h);
    let mag_x = bounds.left() + (bounds.width() - size() as f32) / 2.0;
    let mag_y = bounds.top();
    let radius = size() as f32 / 2.0;
    let cx = mag_x + radius;
    let cy = mag_y + radius;

    let side = ((size() as f32 * scale).round() as u32).max(1);
    let mut zoomed = Pixmap::new(side, side).unwrap();
    let cell = zoom() * scale;
    let magnifier_transform = Transform::from_row(cell, 0.0, 0.0, cell, 0.0, 0.0);
    zoomed.draw_pixmap(
        0,
        0,
        cropped.as_ref(),
        &PixmapPaint::default(),
        magnifier_transform,
        None,
    );

    overlay_crosshair(&mut zoomed, scale);

    let mut mask = tiny_skia::Mask::new(side, side).unwrap();
    let half = side as f32 / 2.0;
    if let Some(circle_path) = PathBuilder::from_circle(half, half, half) {
        mask.fill_path(
            &circle_path,
            tiny_skia::FillRule::Winding,
            true,
            Transform::identity(),
        );
    }
    zoomed.apply_mask(&mask);

    let (px, py) = p.to_px(mag_x, mag_y);
    p.draw_pixmap(px as i32, py as i32, &zoomed);

    let mut stroke = Stroke::default();
    stroke.width = cfg().outline_width;
    if let Some(circle_path) = PathBuilder::from_circle(cx, cy, radius) {
        p.stroke_path(&circle_path, cfg().outline.get(), &stroke);
    }

    super::readout::draw(p, bounds, native, sample_pixel(source, native));
}

/// 【光标放大区】【边缘避让】按 cursor、monitor 和完整 height 返回不越过屏幕边缘的左上角
fn magnifier_position(
    cursor: (f32, f32),
    monitor: (f32, f32, f32, f32),
    height: f32,
) -> (f32, f32) {
    let mag = super::readout::width();
    let (cx, cy) = cursor;
    let (mx, my, mw, mh) = monitor;

    let x = if cx + mag + offset() < mx + mw {
        cx + offset()
    } else {
        cx - mag - offset()
    };

    let y = if cy + height + offset() < my + mh {
        cy + offset()
    } else {
        cy - height - offset()
    };

    (
        x.clamp(mx, (mx + mw - mag).max(mx)),
        y.clamp(my, (my + mh - height).max(my)),
    )
}

/// 【光标放大区】【像素网格】按 scale 向 zoomed 绘制网格与中心像素十字，无返回值
fn overlay_crosshair(zoomed: &mut Pixmap, scale: f32) {
    let cfg = cfg();
    let line = cfg.grid_width * scale;
    let cell = zoom() * scale;
    let w = zoomed.width() as f32;
    let h = zoomed.height() as f32;
    let mut paint = Paint::default();
    paint.anti_alias = false;

    paint.set_color(cfg.grid.color());
    for i in 0..cells() as i32 + 1 {
        let x = i as f32 * cell;
        if let Some(r) = Rect::from_xywh(x, 0.0, line, h) {
            zoomed.fill_path(
                &PathBuilder::from_rect(r),
                &paint,
                tiny_skia::FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
        let y = i as f32 * cell;
        if let Some(r) = Rect::from_xywh(0.0, y, w, line) {
            zoomed.fill_path(
                &PathBuilder::from_rect(r),
                &paint,
                tiny_skia::FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
    }

    paint.set_color(cfg.crosshair.color());
    paint.blend_mode = tiny_skia::BlendMode::SourceOver;
    let center_idx = (cells() / 2) as f32;
    let col_x = center_idx * cell;
    let row_y = center_idx * cell;
    if let Some(r) = Rect::from_xywh(col_x, 0.0, cell, h) {
        zoomed.fill_path(
            &PathBuilder::from_rect(r),
            &paint,
            tiny_skia::FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
    if let Some(r) = Rect::from_xywh(0.0, row_y, w, cell) {
        zoomed.fill_path(
            &PathBuilder::from_rect(r),
            &paint,
            tiny_skia::FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
}
