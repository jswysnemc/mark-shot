//! 【标注选框】【定点栅格】小型圆角和圆形控制点使用固定像素原点，裁切只改变合成范围

use crate::renderer::Painter;
use tiny_skia::{IntRect, Pixmap, Rect};

/// 【标注选框】【稳定定点】按 bounds 和 painter 原始像素相位执行 draw，无返回值，只分配单个控制点的小块缓冲
pub(crate) fn draw(painter: &mut Painter<'_>, bounds: Rect, draw: impl FnOnce(&mut Painter<'_>)) {
    let scale = painter.scale();
    let phase = painter.pixel_phase();
    let pixel = |coordinate: f32, phase: f64| f64::from(coordinate) * f64::from(scale) - phase;
    let Some(bounds) = IntRect::from_ltrb(
        pixel(bounds.left(), phase.0).floor() as i32,
        pixel(bounds.top(), phase.1).floor() as i32,
        pixel(bounds.right(), phase.0).ceil() as i32,
        pixel(bounds.bottom(), phase.1).ceil() as i32,
    ) else {
        return;
    };
    let origin = (
        ((f64::from(bounds.x()) + phase.0) / f64::from(scale)) as f32,
        ((f64::from(bounds.y()) + phase.1) / f64::from(scale)) as f32,
    );
    let position = painter.to_px(origin.0, origin.1);
    // 1. 【标注选框】【可见交集】完全位于本次局部缓冲之外的控制点不分配或绘制
    if position.0.round() >= painter.canvas().width() as f32
        || position.1.round() >= painter.canvas().height() as f32
        || position.0.round() + bounds.width() as f32 <= 0.0
        || position.1.round() + bounds.height() as f32 <= 0.0
    {
        return;
    }
    let Some(mut pixels) = Pixmap::new(bounds.width(), bounds.height()) else {
        return;
    };
    // 2. 【标注选框】【固定绘制】完整小块保留相同圆角采样，父缓冲原点只用于最后的整数像素合成
    draw(&mut painter.sub(&mut pixels).with_origin(origin));
    painter.draw_pixmap(
        position.0.round() as i32,
        position.1.round() as i32,
        &pixels,
    );
}
