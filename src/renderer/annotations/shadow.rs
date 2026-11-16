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

/// 【渲染】【阴影边界】按 stroke_width 返回窗口逻辑像素留量，包含三次滤波支持范围与阴影偏移
pub fn visual_pad(stroke_width: f32) -> f32 {
    let shadow = &crate::config::get().annotations.shadow;
    let halo = (shadow.layers as f32 * shadow.spread / 2.0)
        .max(crate::renderer::blurred_shadow::padding(shadow.blur))
        .max(shadow.pen_halo / 2.0);
    stroke_width / 2.0 + halo + shadow.offset[0].abs().max(shadow.offset[1].abs()) + 2.0
}
/// 【渲染】【操作框边界】无参数，返回操作框命中、描边及阴影所需的窗口逻辑像素留量
pub fn selection_chrome_pad() -> f32 {
    let cfg = crate::config::get();
    let shadow = &cfg.annotations.shadow;
    let halo = (shadow.layers as f32 * shadow.spread / 2.0)
        .max(crate::renderer::blurred_shadow::padding(shadow.blur));
    cfg.input.handle_hit_width / 2.0 + cfg.annotations.handles.width / 2.0 + halo + 2.0
}
