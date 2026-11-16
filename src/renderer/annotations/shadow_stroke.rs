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
    let transform = p.transform();

    let mut paint = Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;

    let mut stroke = Stroke::default();
    stroke.width = stroke_width;
    stroke.line_cap = line_cap;
    stroke.line_join = line_join;

    crate::renderer::paths::blurred_shadow(p, path, Some(&stroke), shadow_color);
    p.canvas.stroke_path(path, &paint, &stroke, transform, None);
}
