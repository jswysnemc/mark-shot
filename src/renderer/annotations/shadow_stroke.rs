//! 【渲染】【阴影笔触】按图形路径绘制柔和阴影

use super::*;

pub(super) fn stroke_with_shadow(
    p: &mut Painter,
    path: &tiny_skia::Path,
    color: Color,
    stroke_width: f32,
    line_cap: LineCap,
    line_join: LineJoin,
    shadow_color: Color,
) {
    let (transform, shadow_transform) = transforms_for(p);

    let mut paint = Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;

    let mut stroke = Stroke::default();
    stroke.width = stroke_width;
    stroke.line_cap = line_cap;
    stroke.line_join = line_join;

    let base_shadow_color = shadow_color;

    stroke_segment_with_shadow(
        p,
        path,
        &paint,
        &stroke,
        base_shadow_color,
        transform,
        shadow_transform,
    );
}
