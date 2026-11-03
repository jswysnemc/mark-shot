//! 【渲染】【文字控制框】绘制文字标注控制边界

use super::*;

pub(super) fn draw_text_box(p: &mut Painter, bbox: &Rect) {
    let cfg = crate::config::get();
    let handles = &cfg.annotations.handles;
    let mut paint = Paint::default();
    paint.set_color(handles.color.get().color());
    paint.anti_alias = true;
    let mut stroke = Stroke::default();
    stroke.width = handles.width;
    stroke.line_cap = tiny_skia::LineCap::Round;
    stroke.line_join = tiny_skia::LineJoin::Round;

    let base_shadow_color = cfg.annotations.shadow.color.color();

    // Selection chrome, not the annotation itself — halo (no offset) reads
    // better than a drop shadow on a rounded-rect outline.
    let transform = p.transform();
    let shadow_transform = transform;
    let pad = cfg.input.handle_hit_width / 2.0;
    let (l, t, ri, b) = (
        bbox.left() - pad,
        bbox.top() - pad,
        bbox.right() + pad,
        bbox.bottom() + pad,
    );

    let (w, h) = (ri - l, b - t);
    let corner_w = (w * handles.corner_length).clamp(handles.min_length.min(w * 0.5), w * 0.5);
    let corner_h = (h * handles.corner_length).clamp(handles.min_length.min(h * 0.5), h * 0.5);
    let r = handles.radius.min(corner_w * 0.5).min(corner_h * 0.5);
    let k = 1.0 - KAPPA;

    let mut pb = PathBuilder::new();

    pb.move_to(l, t + corner_h);
    pb.line_to(l, t + r);
    pb.cubic_to(l, t + r * k, l + r * k, t, l + r, t);
    pb.line_to(l + corner_w, t);
    pb.move_to(ri - corner_w, t);
    pb.line_to(ri - r, t);
    pb.cubic_to(ri - r * k, t, ri, t + r * k, ri, t + r);
    pb.line_to(ri, t + corner_h);
    pb.move_to(ri, b - corner_h);
    pb.line_to(ri, b - r);
    pb.cubic_to(ri, b - r * k, ri - r * k, b, ri - r, b);
    pb.line_to(ri - corner_w, b);
    pb.move_to(l + corner_w, b);
    pb.line_to(l + r, b);
    pb.cubic_to(l + r * k, b, l, b - r * k, l, b - r);
    pb.line_to(l, b - corner_h);

    if let Some(path) = pb.finish() {
        stroke_segment_with_shadow(
            p,
            &path,
            &paint,
            &stroke,
            base_shadow_color,
            transform,
            shadow_transform,
        );
    }
}
