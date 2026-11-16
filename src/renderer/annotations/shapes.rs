//! 【渲染】【框形图形】绘制矩形、椭圆及其填充，线条骨架使用独立模块

use super::*;

pub(super) fn draw_rect(
    p: &mut Painter,
    rect: &Rect,
    color: Color,
    stroke_width: f32,
    shadow_color: Color,
    radius: f32,
) {
    let Some(path) = crate::features::shape::path(*rect, radius, 0.0, (0.0, 0.0)) else {
        return;
    };
    stroke_with_shadow(
        p,
        &path,
        color,
        stroke_width,
        LineCap::Butt,
        LineJoin::Round,
        shadow_color,
    );
}

pub(super) fn draw_circle(
    p: &mut Painter,
    rect: &Rect,
    color: Color,
    stroke_width: f32,
    shadow_color: Color,
) {
    let cx = (rect.left() + rect.right()) / 2.0;
    let cy = (rect.top() + rect.bottom()) / 2.0;
    let rx = rect.width() / 2.0;
    let ry = rect.height() / 2.0;

    if let Some(path) = oval_path(cx, cy, rx, ry) {
        stroke_with_shadow(
            p,
            &path,
            color,
            stroke_width,
            LineCap::Butt,
            LineJoin::Round,
            shadow_color,
        );
    }
}

pub(super) fn fill_rect(
    p: &mut Painter,
    rect: &Rect,
    color: Color,
    stroke_width: f32,
    shadow_color: Color,
    radius: f32,
) {
    let half = stroke_width / 2.0;
    let Some(outer) = Rect::from_ltrb(
        rect.left() - half,
        rect.top() - half,
        rect.right() + half,
        rect.bottom() + half,
    ) else {
        return;
    };
    let Some(path) = crate::features::shape::path(
        outer,
        if radius > 0.0 { radius + half } else { 0.0 },
        0.0,
        (0.0, 0.0),
    ) else {
        return;
    };
    fill_with_shadow(p, &path, color, shadow_color);
}

pub(super) fn fill_circle(
    p: &mut Painter,
    rect: &Rect,
    color: Color,
    stroke_width: f32,
    shadow_color: Color,
) {
    let cx = (rect.left() + rect.right()) / 2.0;
    let cy = (rect.top() + rect.bottom()) / 2.0;
    let rx = rect.width() / 2.0 + stroke_width / 2.0;
    let ry = rect.height() / 2.0 + stroke_width / 2.0;

    if let Some(path) = oval_path(cx, cy, rx, ry) {
        fill_with_shadow(p, &path, color, shadow_color);
    }
}

pub(super) fn fill_with_shadow(
    p: &mut Painter,
    path: &tiny_skia::Path,
    color: Color,
    shadow_color: Color,
) {
    crate::renderer::paths::blurred_shadow(p, path, None, shadow_color);

    let transform = p.transform();
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;
    p.canvas
        .fill_path(path, &paint, FillRule::Winding, transform, None);
}
