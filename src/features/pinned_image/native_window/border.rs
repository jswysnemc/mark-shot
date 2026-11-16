//! 【原生贴图】【预览边框】配置描边绘制在客户区内部，不修改原图或导出字节

use crate::config::Pinned;
use tiny_skia::{Paint, Pixmap, Rect, Transform};

/// 【原生贴图】【边框绘制】按 config 和 scale 在 canvas 四边绘制等宽内边框，无返回值；禁用或非法宽度时不绘制
pub(super) fn draw(canvas: &mut Pixmap, scale: f32, config: &Pinned) {
    if !config.border_enabled || !config.border_width.is_finite() || !scale.is_finite() {
        return;
    }
    let (width, height) = (canvas.width() as f32, canvas.height() as f32);
    let thickness = (config.border_width.max(0.0) * scale.max(0.0)).min(width.min(height) / 2.0);
    if thickness <= 0.0 {
        return;
    }
    let mut paint = Paint::default();
    paint.set_color(config.border_color.color());
    // 1. 【原生贴图】【无重叠填充】左右边排除上下边，半透明颜色在角落只合成一次
    for (x, y, w, h) in [
        (0.0, 0.0, width, thickness),
        (0.0, height - thickness, width, thickness),
        (0.0, thickness, thickness, height - 2.0 * thickness),
        (
            width - thickness,
            thickness,
            thickness,
            height - 2.0 * thickness,
        ),
    ] {
        if let Some(rect) = Rect::from_xywh(x, y, w, h) {
            canvas.fill_rect(rect, &paint, Transform::identity(), None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Rgba;

    /// 【贴图边框测试】【透明角落】无参数，检查分数 DPI 下四边、角落透明度及原图中心，禁用后像素不变，无返回值
    #[test]
    fn preview_border_respects_density_opacity_and_disabled_setting() {
        let mut canvas = Pixmap::new(80, 60).unwrap();
        let mut config = Pinned {
            border_color: Rgba(255, 0, 255, 128),
            ..Default::default()
        };
        draw(&mut canvas, 1.5, &config);
        for (x, y) in [(0, 0), (79, 0), (0, 59), (79, 59), (1, 30), (40, 1)] {
            assert_eq!(canvas.pixel(x, y).unwrap().alpha(), 128);
        }
        assert_eq!(canvas.pixel(40, 30).unwrap().alpha(), 0);
        let original = canvas.clone();
        config.border_enabled = false;
        draw(&mut canvas, 1.5, &config);
        assert_eq!(canvas.data(), original.data());
    }
}
