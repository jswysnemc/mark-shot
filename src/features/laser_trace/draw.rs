//! 【激光】【屏幕绘制】裁切临时绘制区域，避免局部重绘重复叠加透明度

use std::time::Instant;

use tiny_skia::{Color, LineCap, LineJoin, Pixmap, Rect, Stroke};

use super::{LaserStroke, LaserTrace};
use crate::features::annotation_style::window::WindowUnits;
use crate::renderer::{Painter, rect_bounds, scale_rect};

/// 【激光】【局部绘制】将 trace 绘入 canvas；origin、scale 映射逻辑坐标，dirty 限定区域，now 控制消散；无返回值
pub fn draw(
    canvas: &mut Pixmap,
    trace: &LaserTrace,
    origin: (f32, f32),
    dirty: Option<&Rect>,
    scale: f32,
    now: Instant,
) {
    draw_in_window(
        canvas,
        trace,
        origin,
        dirty,
        scale,
        now,
        WindowUnits::default(),
    );
}

/// 【激光】【窗口绘制】按canvas、trace、origin、dirty、scale、now与window绘制当前窗口笔迹，最小主线与白芯尺寸独立于导航，无返回值
pub(crate) fn draw_in_window(
    canvas: &mut Pixmap,
    trace: &LaserTrace,
    origin: (f32, f32),
    dirty: Option<&Rect>,
    scale: f32,
    now: Instant,
    window: WindowUnits,
) {
    if trace.is_empty() || !scale.is_finite() || scale <= 0.0 {
        return;
    }
    let Some(area) = dirty
        .copied()
        .and_then(|rect| scale_rect(&rect, scale))
        .or_else(|| Rect::from_xywh(0.0, 0.0, canvas.width() as f32, canvas.height() as f32))
    else {
        return;
    };
    let Some((x, y, width, height)) = rect_bounds(&area, canvas.width(), canvas.height()) else {
        return;
    };
    let local_origin = (origin.0 + x as f32 / scale, origin.1 + y as f32 / scale);
    let Some(global_area) = Rect::from_xywh(
        local_origin.0,
        local_origin.1,
        width as f32 / scale,
        height as f32 / scale,
    ) else {
        return;
    };
    let visible = |stroke: &&LaserStroke| {
        stroke.path.is_some()
            && stroke.opacity(now) > 0.0
            && stroke.color.alpha() > 0.0
            && stroke
                .bounds()
                .outset(window.canvas(4.0), window.canvas(4.0))
                .and_then(|bounds| bounds.intersect(&global_area))
                .is_some()
    };
    if !trace.strokes().any(|stroke| visible(&stroke)) {
        return;
    }

    // 1. 【激光】【局部绘制】只在已恢复背景的像素区域绘制，未清除区域保持上一帧内容
    let Some(mut temporary) = Pixmap::new(width, height) else {
        return;
    };
    let mut painter = Painter::new(&mut temporary)
        .with_scale(scale)
        .with_chrome_scale(window.canvas(1.0))
        .with_origin(local_origin);
    for stroke in trace.strokes().filter(visible) {
        draw_stroke(&mut painter, stroke, stroke.opacity(now));
    }

    // 2. 【激光】【局部绘制】一次合成三层笔迹，保留分数缩放和跨显示器的坐标映射
    Painter::new(canvas).draw_pixmap(x as i32, y as i32, &temporary);
}

/// 【激光】【分层描边】按 opacity 将 stroke 的辉光、主线和白色细芯绘入 painter；无返回值
fn draw_stroke(painter: &mut Painter<'_>, stroke: &LaserStroke, opacity: f32) {
    let Some(path) = stroke.path.as_ref() else {
        return;
    };
    let width = painter.window_units().stroke(stroke.width, 3.0, None);
    let mut glow = stroke.color;
    glow.apply_opacity(80.0 / 255.0 * opacity);
    let mut core = stroke.color;
    core.apply_opacity(230.0 / 255.0 * opacity);
    let hot = Color::from_rgba(
        1.0,
        1.0,
        1.0,
        170.0 / 255.0 * opacity * stroke.color.alpha(),
    )
    .unwrap();
    for (color, width) in [
        (glow, width * 2.6),
        (core, width),
        (hot, painter.window_units().stroke(width * 0.22, 1.4, None)),
    ] {
        let pen = Stroke {
            width,
            line_cap: LineCap::Round,
            line_join: LineJoin::Round,
            ..Stroke::default()
        };
        painter.stroke_path(path, color, &pen);
    }
}
