//! 【文字标注】【背景绘制】文字背景使用独立透明度、原版四像素圆角，不继承正文透明度

use tiny_skia::{Color, FillRule, Paint, PixmapMut, Rect, Transform};

/// 【文字标注】【背景绘制】将 color 绘入 pixmap 的 bounds；radius 控制圆角，坐标为物理像素，无返回值
pub(super) fn draw(pixmap: &mut PixmapMut<'_>, bounds: Rect, color: Color, radius: f32) {
    if color.alpha() <= 0.0 {
        return;
    }
    if let Some(path) =
        crate::renderer::paths::rounded_rect_path(&bounds, radius, true, true, true, true)
    {
        let mut paint = Paint {
            anti_alias: true,
            ..Default::default()
        };
        paint.set_color(color);
        pixmap.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
}
