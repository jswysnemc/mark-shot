//! 【渲染】【框形图形】绘制矩形、椭圆及其填充，线条骨架使用独立模块

use super::*;

pub(super) fn draw_rect(
    p: &mut Painter,
    rect: &Rect,
    color: Color,
    stroke_width: f32,
    shadow_color: Color,
) {
    let path = PathBuilder::from_rect(*rect);
    stroke_with_shadow(
        p,
        &path,
        color,
        stroke_width,
        LineCap::Butt,
        LineJoin::Miter,
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
    fill_with_shadow(p, &PathBuilder::from_rect(outer), color, shadow_color);
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
    let (transform, shadow_transform) = transforms_for(p);

    let mut shadow_paint = Paint::default();
    shadow_paint.set_color(shadow_color);
    shadow_paint.anti_alias = true;
    p.canvas.fill_path(
        path,
        &shadow_paint,
        FillRule::Winding,
        shadow_transform,
        None,
    );

    let mut paint = Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;
    p.canvas
        .fill_path(path, &paint, FillRule::Winding, transform, None);
}
