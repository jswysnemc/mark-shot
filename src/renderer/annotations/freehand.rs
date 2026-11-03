//! 【渲染】【自由笔迹】绘制平滑画笔与荧光笔

use super::*;
use crate::renderer::stroke_path::freehand_path;

pub(super) fn draw_pen(
    p: &mut Painter,
    points: &[(f32, f32)],
    color: Color,
    stroke_width: f32,
    shadow_color: Color,
) {
    let Some(path) = freehand_path(points, 0, None) else {
        return;
    };

    let transform = p.transform();
    // 1. drawing a halo around the full path
    let mut halo_paint = Paint::default();
    halo_paint.set_color(shadow_color);
    halo_paint.anti_alias = true;
    let mut halo_stroke = Stroke::default();
    halo_stroke.width = stroke_width + crate::config::get().annotations.shadow.pen_halo;
    halo_stroke.line_cap = LineCap::Round;
    halo_stroke.line_join = LineJoin::Round;

    // 2. normal stroke paint
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;
    let mut stroke = Stroke::default();
    stroke.width = stroke_width;
    stroke.line_cap = LineCap::Round;
    stroke.line_join = LineJoin::Round;

    // draw halo first
    p.canvas
        .stroke_path(&path, &halo_paint, &halo_stroke, transform, None);
    // then normal stroke
    p.canvas
        .stroke_path(&path, &paint, &stroke, transform, None);
}

pub(super) fn highlighter_color(mut color: Color) -> Color {
    color.apply_opacity(crate::config::get().tools.highlighter.opacity);
    color
}

pub(super) fn draw_highlighter(
    p: &mut Painter,
    points: &[(f32, f32)],
    flat_ends: [bool; 2],
    color: Color,
    stroke_width: f32,
) {
    if let Some(path) = freehand_path(points, 0, None) {
        stroke_highlighter(p, &path, points, flat_ends, color, stroke_width);
    }
}

pub(super) fn stroke_highlighter(
    p: &mut Painter,
    path: &tiny_skia::Path,
    points: &[(f32, f32)],
    flat_ends: [bool; 2],
    color: Color,
    stroke_width: f32,
) {
    let flat_ends = if points.windows(2).all(|w| w[0] == w[1]) {
        [false; 2]
    } else {
        flat_ends
    };
    let transform = p.transform();
    let mut paint = Paint::default();
    paint.set_color(highlighter_color(color));
    paint.anti_alias = true;
    let mut stroke = Stroke::default();
    stroke.width = stroke_width;
    stroke.line_cap = if flat_ends == [false; 2] {
        LineCap::Round
    } else {
        LineCap::Butt
    };
    stroke.line_join = LineJoin::Round;

    let round_end = match flat_ends {
        [true, false] => points.last(),
        [false, true] => points.first(),
        _ => None,
    };
    let Some(&(x, y)) = round_end else {
        p.canvas.stroke_path(path, &paint, &stroke, transform, None);
        return;
    };

    let (cw, ch) = (p.canvas.width(), p.canvas.height());
    let (Some(mut mask), Some(outline), Some(dot), Some(rect)) = (
        Mask::new(cw, ch),
        path.stroke(&stroke, p.scale()),
        PathBuilder::from_circle(x, y, stroke_width / 2.0),
        Rect::from_xywh(0.0, 0.0, cw as f32, ch as f32),
    ) else {
        return;
    };
    mask.fill_path(&outline, FillRule::Winding, true, transform);
    mask.fill_path(&dot, FillRule::Winding, true, transform);
    p.canvas
        .fill_rect(rect, &paint, Transform::identity(), Some(&mask));
}

pub fn draw_highlighter_live(
    p: &mut Painter,
    points: &[(f32, f32)],
    flat_ends: [bool; 2],
    color: Color,
    stroke_width: f32,
    dirty: Option<&Rect>,
) {
    let (cw, ch) = (p.canvas.width(), p.canvas.height());
    let s = p.scale();
    let Some((x, y, w, h)) = dirty
        .and_then(|r| scale_rect(r, s))
        .or_else(|| Rect::from_xywh(0.0, 0.0, cw as f32, ch as f32))
        .and_then(|r| rect_bounds(&r, cw, ch))
    else {
        return;
    };
    let Some(mut tmp) = Pixmap::new(w, h) else {
        return;
    };

    let origin = (p.origin.0 + x as f32 / s, p.origin.1 + y as f32 / s);
    let pad = stroke_width / 2.0 + 1.0;
    let clip = Rect::from_xywh(
        origin.0 - pad,
        origin.1 - pad,
        w as f32 / s + pad * 2.0,
        h as f32 / s + pad * 2.0,
    );
    let Some(path) = freehand_path(points, 0, clip) else {
        return;
    };
    stroke_highlighter(
        &mut p.sub(&mut tmp).with_origin(origin),
        &path,
        points,
        flat_ends,
        color,
        stroke_width,
    );

    p.draw_pixmap(x as i32, y as i32, &tmp);
}

pub fn stroke_pen_segment(
    p: &mut Painter,
    path: &tiny_skia::Path,
    color: Color,
    stroke_width: f32,
) {
    let transform = p.transform();
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;
    let mut stroke = Stroke::default();
    stroke.width = stroke_width;
    stroke.line_cap = LineCap::Round;
    stroke.line_join = LineJoin::Round;

    p.canvas.stroke_path(path, &paint, &stroke, transform, None);
}

pub fn draw_pen_active_tail(
    p: &mut Painter,
    points: &[(f32, f32)],
    baked: usize,
    color: Color,
    stroke_width: f32,
) {
    if let Some(path) = freehand_path(points, baked, None) {
        stroke_pen_segment(p, &path, color, stroke_width);
    }
}
