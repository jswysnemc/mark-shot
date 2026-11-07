//! 【标注旋转】【固定分块】斜角正文按原始物理像素网格栅格化，局部缓冲只合成相交部分

use super::Painter;
use crate::types::Annotation;
use tiny_skia::{Pixmap, PixmapPaint, Transform};

const SIDE: i64 = 128;

/// 【标注旋转】【物理坐标】按 value、scale 与 phase 返回原始网格整数原点，不累积局部逻辑坐标舍入
fn pixel(value: f32, scale: f64, phase: f64) -> i64 {
    (f64::from(value) * scale - phase).round() as i64
}

/// 【标注旋转】【区域绘制】按 painter 和 annotation 在固定网格执行 body 并合成，无返回值，每个临时图像最多128×128像素
pub(super) fn draw(
    painter: &mut Painter<'_>,
    annotation: &Annotation,
    body: impl FnMut(&mut Painter<'_>),
) {
    draw_region(painter, annotation.damage_bbox(false), body);
}

/// 【标注绘制】【固定区域】按painter、bounds和body绘制固定物理网格，任意局部裁切保持相同栅格结果，无返回值
pub(crate) fn draw_region(
    painter: &mut Painter<'_>,
    bounds: tiny_skia::Rect,
    mut body: impl FnMut(&mut Painter<'_>),
) {
    let scale = f64::from(painter.scale());
    let phase = painter.pixel_phase();
    let origin = painter.origin();
    let origin = (
        pixel(origin.0, scale, phase.0),
        pixel(origin.1, scale, phase.1),
    );
    let canvas = painter.canvas();
    let size = (i64::from(canvas.width()), i64::from(canvas.height()));
    let from = (
        ((f64::from(bounds.left()) * scale - phase.0).floor() as i64).max(origin.0),
        ((f64::from(bounds.top()) * scale - phase.1).floor() as i64).max(origin.1),
    );
    let to = (
        ((f64::from(bounds.right()) * scale - phase.0).ceil() as i64).min(origin.0 + size.0),
        ((f64::from(bounds.bottom()) * scale - phase.1).ceil() as i64).min(origin.1 + size.1),
    );
    if from.0 >= to.0 || from.1 >= to.1 {
        return;
    }
    let Some(mut tile) = Pixmap::new(SIDE as u32, SIDE as u32) else {
        return;
    };
    // 1. 【标注旋转】【固定网格】所有入口使用相同块原点、范围和抗锯齿计算，损坏区域不会改变正文像素
    for y in (from.1.div_euclid(SIDE)..=(to.1 - 1).div_euclid(SIDE)).map(|index| index * SIDE) {
        for x in (from.0.div_euclid(SIDE)..=(to.0 - 1).div_euclid(SIDE)).map(|index| index * SIDE) {
            tile.fill(tiny_skia::Color::TRANSPARENT);
            body(&mut painter.sub(&mut tile).with_origin((
                ((x as f64 + phase.0) / scale) as f32,
                ((y as f64 + phase.1) / scale) as f32,
            )));
            // 2. 【标注旋转】【整像素合成】每块只覆盖自身范围，负原点和局部缓冲交由目标画布裁切，透明正文只合成一次
            painter.canvas().draw_pixmap(
                (x - origin.0) as i32,
                (y - origin.1) as i32,
                tile.as_ref(),
                &PixmapPaint::default(),
                Transform::identity(),
                None,
            );
        }
    }
}
