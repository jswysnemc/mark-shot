use crate::config::Magnifier;
use crate::renderer::Painter;
use crate::renderer::paths::rounded_rect_path;
use crate::renderer::text::TextStyle;
use crate::theme::{font, radius};
use crate::types::Capture;
use crate::ui::magnifier::{cells, offset, sample_pixel, size, zoom};
use tiny_skia::{Color, Paint, PathBuilder, Pixmap, PixmapPaint, Rect, Stroke, Transform};

fn cfg() -> &'static Magnifier {
    &crate::config::get().magnifier
}

/// Loupe plus, for the eyedropper, the colour plate under it.
pub fn magnifier_rect(
    cursor: (f32, f32),
    monitor_w: f32,
    monitor_h: f32,
    with_label: bool,
) -> Rect {
    let height = box_height(with_label);
    let (mag_x, mag_y) = magnifier_position(cursor, (0.0, 0.0, monitor_w, monitor_h), height);
    Rect::from_xywh(mag_x, mag_y, size() as f32, height).unwrap()
}

fn box_height(with_label: bool) -> f32 {
    if with_label {
        size() as f32 + cfg().label_gap + cfg().label_height
    } else {
        size() as f32
    }
}

/// `label` is the colour plate the eyedropper needs; without it only the loupe is drawn.
pub fn draw_magnifier(
    p: &mut Painter,
    capture: &Capture,
    cursor: (f32, f32),
    (screen_w, screen_h): (f32, f32),
    label: bool,
) {
    let scale = p.scale();
    let source = &capture.pixmap;
    let native = capture.to_native((cursor.0 as f64, cursor.1 as f64));

    let sample_size = cells() as i32;

    let half = (cells() / 2) as i32;
    let src_x = (native.0 as i32 - half)
        .max(0)
        .min(source.width() as i32 - sample_size) as u32;
    let src_y = (native.1 as i32 - half)
        .max(0)
        .min(source.height() as i32 - sample_size) as u32;

    let mut cropped = Pixmap::new(sample_size as u32, sample_size as u32).unwrap();
    cropped.draw_pixmap(
        -(src_x as i32),
        -(src_y as i32),
        source.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        None,
    );

    let (mag_x, mag_y) =
        magnifier_position(cursor, (0.0, 0.0, screen_w, screen_h), box_height(label));
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

    if label && let Some(color) = sample_pixel(source, native) {
        draw_color_label(p, mag_x, mag_y + size() as f32 + cfg().label_gap, color);
    }
}

fn draw_color_label(p: &mut Painter, x: f32, y: f32, color: Color) {
    let cfg = cfg();
    let width = size() as f32;
    let Some(plate) = Rect::from_xywh(x, y, width, cfg.label_height) else {
        return;
    };
    p.fill_rounded(plate, radius::panel(), cfg.label_background.get());
    stroke_outline(p, plate, radius::panel());

    let text = crate::ui::settings_panel::ValueField::Hex.text(color);
    let text_width = p.measure(&text, font::label());
    let (swatch_size, swatch_gap) = (cfg.swatch_size, cfg.swatch_gap);
    let group = swatch_size + swatch_gap + text_width;
    let swatch_x = x + ((width - group) / 2.0).max(swatch_gap);

    if let Some(swatch) = Rect::from_xywh(
        swatch_x,
        y + (cfg.label_height - swatch_size) / 2.0,
        swatch_size,
        swatch_size,
    ) {
        p.fill_rounded(swatch, cfg.swatch_radius, color);
        stroke_outline(p, swatch, cfg.swatch_radius);
    }

    if let Some(text_rect) = Rect::from_xywh(
        swatch_x + swatch_size + swatch_gap,
        y,
        (width - (swatch_x - x) - swatch_size - swatch_gap).max(0.0),
        cfg.label_height,
    ) {
        p.text(
            text_rect,
            &text,
            TextStyle::new(font::label(), cfg.label_text.get().color()),
        );
    }
}

fn stroke_outline(p: &mut Painter, rect: Rect, radius: f32) {
    let outline = cfg().outline_width;
    let inset = outline / 2.0;
    let Some(inner) = Rect::from_xywh(
        rect.left() + inset,
        rect.top() + inset,
        (rect.width() - outline).max(0.1),
        (rect.height() - outline).max(0.1),
    ) else {
        return;
    };
    let Some(path) = rounded_rect_path(&inner, (radius - inset).max(0.0), true, true, true, true)
    else {
        return;
    };
    let stroke = Stroke {
        width: outline,
        ..Stroke::default()
    };
    p.stroke_path(&path, cfg().outline.get(), &stroke);
}

fn magnifier_position(
    cursor: (f32, f32),
    monitor: (f32, f32, f32, f32),
    height: f32,
) -> (f32, f32) {
    let mag = size() as f32;
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

    (x, y)
}

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
