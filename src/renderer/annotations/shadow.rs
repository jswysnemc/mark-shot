//! 【渲染】【阴影参数】统一颜色透明度、变换和损坏边界

use super::*;

/// Scales the shadow's own fixed alpha by the annotation color's alpha,
/// so a half-transparent stroke doesn't end up with a full-opacity shadow
/// sitting underneath it.
pub(super) fn shadow_alpha_for(color: Color) -> u8 {
    (crate::config::get().annotations.shadow.color.alpha() as f32 * color.alpha()) as u8
}

pub fn shadow_color_for(color: Color) -> Color {
    let shadow = crate::config::get().annotations.shadow.color;
    shadow.with_alpha(shadow_alpha_for(color)).color()
}

/// Builds the "real" transform plus the same transform shifted by
/// `annotations.shadow.offset`, from the painter's viewport. Keeps every shape's
/// shadow offset in sync without repeating the translate math.
pub(super) fn transforms_for(p: &Painter) -> (Transform, Transform) {
    let [dx, dy] = crate::config::get().annotations.shadow.offset;
    (p.transform(), p.transform_shifted(dx, dy))
}

// Only matters for the offset-shadow path (stroke_with_shadow), not halo
// (draw_text_box / draw_annotation_handles, which don't use shadow.offset
// at all — their layers spread evenly on every side by design).
//
// Keep (shadow.layers * shadow.spread) / 2.0 <= shadow.offset[1] if you
// tune these: that's what keeps the widest, faintest layer's natural top
// overhang from poking above the shape. Currently 4 * 1.5 / 2 = 3.0,
// exactly matching shadow.offset[1] = 3.0 — zero headroom, so don't shrink
// the offset or grow the spread without adjusting the other side too.

pub fn visual_pad(stroke_width: f32) -> f32 {
    let shadow = &crate::config::get().annotations.shadow;
    let spread = (shadow.layers as f32 * shadow.spread).max(shadow.pen_halo);
    let max_stroke_extent = (stroke_width + spread) / 2.0;
    max_stroke_extent + shadow.offset[0].abs().max(shadow.offset[1].abs()) + 2.0
}
pub fn selection_chrome_pad() -> f32 {
    let cfg = crate::config::get();
    let shadow = &cfg.annotations.shadow;
    let max_stroke_extent =
        (cfg.annotations.handles.width + shadow.layers as f32 * shadow.spread) / 2.0;
    cfg.input.handle_hit_width / 2.0 + max_stroke_extent + 2.0
}

pub(super) fn stroke_segment_with_shadow(
    p: &mut Painter,
    path: &tiny_skia::Path,
    paint: &Paint,
    stroke: &Stroke,
    base_shadow_color: Color,
    transform: Transform,
    shadow_transform: Transform,
) {
    let shadow = &crate::config::get().annotations.shadow;
    for i in (1..=shadow.layers).rev() {
        let mut layer_stroke = stroke.clone();

        let extra_width = i as f32 * shadow.spread;
        layer_stroke.width = stroke.width + extra_width;

        let alpha_factor = 1.0 / (1.0 + (i as f32 * shadow.falloff));
        let current_alpha = (base_shadow_color.alpha() * alpha_factor).clamp(0.0, 1.0);

        if let Some(layer_color) = Color::from_rgba(
            base_shadow_color.red(),
            base_shadow_color.green(),
            base_shadow_color.blue(),
            current_alpha,
        ) {
            let mut layer_paint = Paint::default();
            layer_paint.set_color(layer_color);
            layer_paint.anti_alias = true;

            p.canvas
                .stroke_path(path, &layer_paint, &layer_stroke, shadow_transform, None);
        }
    }

    p.canvas.stroke_path(path, paint, stroke, transform, None);
}
