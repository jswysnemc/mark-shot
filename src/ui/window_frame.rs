//! 【窗口外观】【统一边框】独立窗口共用内嵌细边框与分层圆角外角裁切，浅色和深色背景均保留清晰轮廓

use crate::{
    renderer::{Painter, paths::rounded_rect_path},
    theme::{Rgba, window::Palette},
};
use tiny_skia::{BlendMode, FillRule, Paint, Path, PathBuilder, Pixmap, Rect, Stroke, Transform};

const OUTER_RADIUS: f32 = 10.0;
const STROKE_INSET: f32 = 0.5;
const STROKE_RADIUS: f32 = OUTER_RADIUS - STROKE_INSET;

/// 【窗口外观】【边框颜色】返回当前明暗主题下的窗口外框描边颜色，无参数
fn border_color() -> Rgba {
    if crate::theme::Mode::current() == crate::theme::Mode::Light {
        Rgba(148, 163, 184, 255)
    } else {
        Rgba(100, 116, 139, 255)
    }
}

/// 【窗口外观】【奇偶外角路径】按逻辑宽高构造画布外框与圆角内框组成的奇偶环路，供外角透明裁切使用
fn exterior_corner_mask(width: f32, height: f32) -> Option<Path> {
    let outer = Rect::from_xywh(0.0, 0.0, width, height)?;
    let inner = Rect::from_xywh(
        STROKE_INSET,
        STROKE_INSET,
        width - STROKE_INSET * 2.0,
        height - STROKE_INSET * 2.0,
    )?;
    let r = STROKE_RADIUS
        .min(inner.width() / 2.0)
        .min(inner.height() / 2.0);
    let k = 1.0 - crate::renderer::paths::KAPPA;
    let (l, t, ri, b) = (inner.left(), inner.top(), inner.right(), inner.bottom());

    let mut pb = PathBuilder::new();
    pb.push_rect(outer);
    pb.move_to(l + r, t);
    pb.line_to(ri - r, t);
    pb.cubic_to(ri - r * k, t, ri, t + r * k, ri, t + r);
    pb.line_to(ri, b - r);
    pb.cubic_to(ri, b - r * k, ri - r * k, b, ri - r, b);
    pb.line_to(l + r, b);
    pb.cubic_to(l + r * k, b, l, b - r * k, l, b - r);
    pb.line_to(l, t + r);
    pb.cubic_to(l, t + r * k, l + r * k, t, l + r, t);
    pb.close();
    pb.finish()
}

/// 【窗口外观】【边框绘制】按 canvas 的实际像素尺寸与 scale 绘制圆角窗口轮廓，保留已有内容，无返回值
pub(crate) fn draw(canvas: &mut Pixmap, scale: f32) {
    let scale = scale.max(0.1);
    let width = canvas.width() as f32 / scale;
    let height = canvas.height() as f32 / scale;
    let palette = Palette::current();
    let border = border_color();
    let Some(bounds) = Rect::from_xywh(
        STROKE_INSET,
        STROKE_INSET,
        width - STROKE_INSET * 2.0,
        height - STROKE_INSET * 2.0,
    ) else {
        return;
    };
    let Some(path) = rounded_rect_path(&bounds, STROKE_RADIUS, true, true, true, true) else {
        return;
    };
    let mut painter = Painter::new(canvas).with_scale(scale);
    painter.stroke_path(
        &path,
        border,
        &Stroke {
            width: 1.0,
            ..Default::default()
        },
    );
    if let Some(inner) = Rect::from_xywh(1.5, 1.5, width - 3.0, height - 3.0)
        && let Some(path) = rounded_rect_path(&inner, STROKE_RADIUS - 1.0, true, true, true, true)
    {
        painter.stroke_path(
            &path,
            palette.window,
            &Stroke {
                width: 0.5,
                ..Default::default()
            },
        );
    }
}

/// 【窗口外观】【直角细边】为非分层系统窗口按 canvas 与 scale 绘制贴合边缘的矩形细边框，避免内缩圆角露出底色直角
pub(crate) fn draw_rectangular(canvas: &mut Pixmap, scale: f32) {
    let scale = scale.max(0.1);
    let width = canvas.width() as f32 / scale;
    let height = canvas.height() as f32 / scale;
    let Some(bounds) = Rect::from_xywh(
        STROKE_INSET,
        STROKE_INSET,
        width - STROKE_INSET * 2.0,
        height - STROKE_INSET * 2.0,
    ) else {
        return;
    };
    let mut painter = Painter::new(canvas).with_scale(scale);
    painter.stroke_path(
        &PathBuilder::from_rect(bounds),
        border_color(),
        &Stroke {
            width: 1.0,
            ..Default::default()
        },
    );
}

/// 【窗口外观】【圆角外角裁切】在支持逐像素透明度的独立窗口表面擦除圆角外侧直角像素并重绘抗锯齿细边框，无返回值
pub(crate) fn clip_rounded_surface(canvas: &mut Pixmap, scale: f32) {
    let scale = scale.max(0.1);
    let width = canvas.width() as f32 / scale;
    let height = canvas.height() as f32 / scale;
    // 1. 先按描边中心线擦除圆角外侧的全部不透明背景像素，消除四角直角残留与亮色光晕
    if let Some(mask) = exterior_corner_mask(width, height) {
        let mut paint = Paint::default();
        paint.set_color_rgba8(255, 255, 255, 255);
        paint.anti_alias = true;
        paint.blend_mode = BlendMode::DestinationOut;
        canvas.fill_path(
            &mask,
            &paint,
            FillRule::EvenOdd,
            Transform::from_scale(scale, scale),
            None,
        );
    }
    // 2. 在透明外缘与不透明内底交界处重新描摹圆角边框，保证内外两侧边缘平滑过渡
    draw(canvas, scale);
}

#[cfg(test)]
mod tests {
    use super::*;
    use tiny_skia::Color;

    /// 【窗口外观测试】【四角透明裁切】验证多倍率下圆角外侧直角像素全部清零且窗口内部与边框保持不透明
    #[test]
    fn clip_rounded_surface_clears_all_four_right_angle_corners_across_scales() {
        for scale in [1.0_f32, 1.25, 1.5, 2.0] {
            let pw = (240.0 * scale).round() as u32;
            let ph = (160.0 * scale).round() as u32;
            let mut canvas = Pixmap::new(pw, ph).unwrap();
            canvas.fill(Color::WHITE);
            draw(&mut canvas, scale);
            clip_rounded_surface(&mut canvas, scale);

            for (x, y) in [
                (0, 0),
                (1, 1),
                (pw - 1, 0),
                (pw - 2, 1),
                (0, ph - 1),
                (1, ph - 2),
                (pw - 1, ph - 1),
                (pw - 2, ph - 2),
            ] {
                assert_eq!(
                    canvas.pixel(x, y).unwrap().alpha(),
                    0,
                    "corner ({x}, {y}) at scale {scale} must be transparent"
                );
            }
            let center = canvas.pixel(pw / 2, ph / 2).unwrap();
            assert_eq!(center.alpha(), 255);
            assert_eq!(center.red(), 255);
        }
    }
}
