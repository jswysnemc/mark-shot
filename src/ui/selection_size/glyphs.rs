//! 【选区尺寸】【矢量图标】复用原版几何比例绘制路径，图标不包含位图资源或文字符号

use crate::{renderer::Painter, theme::Rgba};
use tiny_skia::{LineCap, LineJoin, Path, PathBuilder, Rect, Stroke, StrokeDash};

/// 【选区尺寸】【比例框路径】按 frame 返回从左侧圆角切点开始的原版圆角路径，起点决定虚线相位
fn ratio_frame(frame: Rect) -> Option<Path> {
    let radius = 1.5_f32.min(frame.width() / 2.0).min(frame.height() / 2.0);
    let k = radius * (1.0 - crate::renderer::paths::KAPPA);
    let (left, top, right, bottom) = (frame.left(), frame.top(), frame.right(), frame.bottom());
    let mut path = PathBuilder::new();
    path.move_to(left, top + radius);
    path.cubic_to(left, top + k, left + k, top, left + radius, top);
    path.line_to(right - radius, top);
    path.cubic_to(right - k, top, right, top + k, right, top + radius);
    path.line_to(right, bottom - radius);
    path.cubic_to(right, bottom - k, right - k, bottom, right - radius, bottom);
    path.line_to(left + radius, bottom);
    path.cubic_to(left + k, bottom, left, bottom - k, left, bottom - radius);
    path.close();
    path.finish()
}

/// 【选区尺寸】【锁形图标】在 p 的 box 内按 locked 和 ink 绘制锁体与锁梁，无返回值
pub fn lock(p: &mut Painter, box_rect: Rect, locked: bool, ink: Rgba) {
    let unit = box_rect.width().min(box_rect.height());
    let x = box_rect.left() + (box_rect.width() - unit) / 2.0;
    let y = box_rect.top() + (box_rect.height() - unit) / 2.0;
    // 1. 【选区尺寸】【锁体】下半部使用同色圆角实体，不增加装饰边线
    let body = Rect::from_xywh(x + unit * 0.18, y + unit * 0.46, unit * 0.64, unit * 0.46).unwrap();
    p.fill_rounded(body, unit * 0.1, ink);
    // 2. 【选区尺寸】【锁梁】上半圆与两端保持原版圆头线宽，打开时右端抬升
    let width = unit * 0.4;
    let left = x + unit * 0.5 - width / 2.0;
    let top = y + unit * 0.08;
    let radius = width / 2.0;
    let k = radius * 0.552_284_8;
    let mut path = PathBuilder::new();
    path.move_to(left, body.top());
    path.line_to(left, top + radius);
    path.cubic_to(
        left,
        top + radius - k,
        left + radius - k,
        top,
        left + radius,
        top,
    );
    path.cubic_to(
        left + radius + k,
        top,
        left + width,
        top + radius - k,
        left + width,
        top + radius,
    );
    path.line_to(
        left + width,
        body.top() - if locked { 0.0 } else { unit * 0.14 },
    );
    if let Some(path) = path.finish() {
        p.stroke_path(
            &path,
            ink.color(),
            &Stroke {
                width: 1.4_f32.max(unit * 0.12),
                line_cap: LineCap::Round,
                ..Stroke::default()
            },
        );
    }
}

/// 【选区尺寸】【比例图标】在 p 的 box_rect 内按 ratio 和 ink 绘制比例框，自由比例为虚线，无返回值
pub fn ratio(p: &mut Painter, box_rect: Rect, ratio: f64, ink: Rgba) {
    let free = ratio <= 0.0;
    let frame = if free {
        Rect::from_xywh(
            box_rect.left() + box_rect.width() * 0.12,
            box_rect.top() + box_rect.height() * 0.12,
            box_rect.width() * 0.76,
            box_rect.height() * 0.76,
        )
    } else {
        let mut width = box_rect.width() - 2.0;
        let mut height = width / ratio as f32;
        if height > box_rect.height() - 2.0 {
            height = box_rect.height() - 2.0;
            width = height * ratio as f32;
        }
        Rect::from_xywh(
            box_rect.left() + (box_rect.width() - width) / 2.0,
            box_rect.top() + (box_rect.height() - height) / 2.0,
            width,
            height,
        )
    };
    if let Some(frame) = frame
        && let Some(path) = ratio_frame(frame)
    {
        p.stroke_path(
            &path,
            ink.color(),
            &Stroke {
                width: 1.3,
                line_cap: LineCap::Square,
                line_join: LineJoin::Bevel,
                dash: free.then(|| StrokeDash::new(vec![2.6, 2.6], 0.0).unwrap()),
                ..Stroke::default()
            },
        );
    }
}

/// 【选区尺寸】【折线路径】在 p 按 points 和 width 绘制 ink 圆角折线，无返回值
fn polyline(p: &mut Painter, points: &[(f32, f32)], width: f32, ink: Rgba) {
    let Some(&(x, y)) = points.first() else {
        return;
    };
    let mut path = PathBuilder::new();
    path.move_to(x, y);
    for &(x, y) in &points[1..] {
        path.line_to(x, y);
    }
    if let Some(path) = path.finish() {
        p.stroke_path(
            &path,
            ink.color(),
            &Stroke {
                width,
                line_cap: LineCap::Round,
                line_join: LineJoin::Round,
                ..Stroke::default()
            },
        );
    }
}

/// 【选区尺寸】【下拉箭头】在 p 的 box_rect 内按 ink 绘制原版向下折线，无返回值
pub fn chevron(p: &mut Painter, box_rect: Rect, ink: Rgba) {
    let x = box_rect.left() + box_rect.width() / 2.0;
    let y = box_rect.top() + box_rect.height() / 2.0;
    let half = box_rect.width().min(box_rect.height()) * 0.24;
    polyline(
        p,
        &[
            (x - half, y - half / 2.0),
            (x, y + half / 2.0),
            (x + half, y - half / 2.0),
        ],
        1.4,
        ink,
    );
}

/// 【选区尺寸】【选中勾选】在 p 的 box_rect 内按 ink 绘制当前预设标记，无返回值
pub fn check(p: &mut Painter, box_rect: Rect, ink: Rgba) {
    let unit = box_rect.width().min(box_rect.height());
    let x = box_rect.left() + (box_rect.width() - unit) / 2.0;
    let y = box_rect.top() + (box_rect.height() - unit) / 2.0;
    polyline(
        p,
        &[
            (x + unit * 0.2, y + unit * 0.52),
            (x + unit * 0.42, y + unit * 0.72),
            (x + unit * 0.8, y + unit * 0.3),
        ],
        1.6,
        ink,
    );
}
