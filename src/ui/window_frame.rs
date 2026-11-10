//! 【窗口外观】【统一边框】独立窗口共用内嵌细边框，浅色和深色背景均保留可见轮廓

use crate::{
    renderer::{Painter, paths::rounded_rect_path},
    theme::{Rgba, window::Palette},
};
use tiny_skia::{Pixmap, Rect, Stroke};

/// 【窗口外观】【边框绘制】按 canvas 的实际像素尺寸与 scale 绘制窗口轮廓，保留已有内容，返回值为空
pub(crate) fn draw(canvas: &mut Pixmap, scale: f32) {
    let width = canvas.width() as f32 / scale;
    let height = canvas.height() as f32 / scale;
    let palette = Palette::current();
    let border = if crate::theme::Mode::current() == crate::theme::Mode::Light {
        Rgba(148, 163, 184, 255)
    } else {
        Rgba(100, 116, 139, 255)
    };
    let Some(bounds) = Rect::from_xywh(0.75, 0.75, width - 1.5, height - 1.5) else {
        return;
    };
    let Some(path) = rounded_rect_path(&bounds, 10.0, true, true, true, true) else {
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
    if let Some(inner) = Rect::from_xywh(1.75, 1.75, width - 3.5, height - 3.5)
        && let Some(path) = rounded_rect_path(&inner, 9.0, true, true, true, true)
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
