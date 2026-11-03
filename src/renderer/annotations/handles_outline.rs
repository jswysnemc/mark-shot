//! 【渲染】【控制手柄】绘制标注编辑手柄

use super::*;

/// 【标注渲染】【默认控制框】把 bbox 自动增加命中留白后绘入 p；保持已有工具的外框行为，无返回值
pub(super) fn draw_annotation_handles(p: &mut Painter, bbox: &Rect) {
    let padding = crate::config::get().input.handle_hit_width * 0.5;
    if let Some(frame) = bbox.outset(padding, padding) {
        draw_exact_frame(p, &frame);
    }
}

/// 【标注渲染】【精确控制框】将已包含留白的 bbox 外框绘入 p；不再次扩大位置，保留统一描边和阴影，无返回值
pub(super) fn draw_exact_frame(p: &mut Painter, bbox: &Rect) {
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

    // Eight small, disjoint segments — halo (no offset) instead of a drop shadow.
    let transform = p.transform();
    let shadow_transform = transform;

    let l = bbox.left();
    let t = bbox.top();
    let ri = bbox.right();
    let b = bbox.bottom();

    let w = ri - l;
    let h = b - t;
    let mid_x = (l + ri) / 2.0;
    let mid_y = (t + b) / 2.0;

    // Mid handles
    let mid_hw = (w * handles.middle_length).clamp(handles.min_length.min(w * 0.5), w * 0.5);
    let mid_hh = (h * handles.middle_length).clamp(handles.min_length.min(h * 0.5), h * 0.5);

    // Edges
    let corner_w = (w * handles.corner_length).clamp(handles.min_length.min(w * 0.5), w * 0.5);
    let corner_h = (h * handles.corner_length).clamp(handles.min_length.min(h * 0.5), h * 0.5);

    let r = handles.radius.min(corner_w * 0.5).min(corner_h * 0.5);
    let k = 1.0 - KAPPA;

    let mut pb = PathBuilder::new();

    // Top-Left
    pb.move_to(l, t + corner_h);
    pb.line_to(l, t + r);
    pb.cubic_to(l, t + r * k, l + r * k, t, l + r, t);
    pb.line_to(l + corner_w, t);

    // Top-Right
    pb.move_to(ri - corner_w, t);
    pb.line_to(ri - r, t);
    pb.cubic_to(ri - r * k, t, ri, t + r * k, ri, t + r);
    pb.line_to(ri, t + corner_h);

    // Bottom-Right
    pb.move_to(ri, b - corner_h);
    pb.line_to(ri, b - r);
    pb.cubic_to(ri, b - r * k, ri - r * k, b, ri - r, b);
    pb.line_to(ri - corner_w, b);

    // Bottom-Left
    pb.move_to(l + corner_w, b);
    pb.line_to(l + r, b);
    pb.cubic_to(l + r * k, b, l, b - r * k, l, b - r);
    pb.line_to(l, b - corner_h);

    // Top middle
    pb.move_to(mid_x - mid_hw / 2.0, t);
    pb.line_to(mid_x + mid_hw / 2.0, t);

    // Bottom middle
    pb.move_to(mid_x - mid_hw / 2.0, b);
    pb.line_to(mid_x + mid_hw / 2.0, b);

    // Left middle
    pb.move_to(l, mid_y - mid_hh / 2.0);
    pb.line_to(l, mid_y + mid_hh / 2.0);

    // Right middle
    pb.move_to(ri, mid_y - mid_hh / 2.0);
    pb.line_to(ri, mid_y + mid_hh / 2.0);

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
