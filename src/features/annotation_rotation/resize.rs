//! 【普通标注】【缩放边界】移植原版未旋转矩形的八方向缩放、等比联动与原图边界约束

use super::geometry::{Point, center};
use crate::types::SelectionHandle;
use tiny_skia::Rect;

/// 【普通标注】【有界比例】按 raw、maximum 和 minimum 返回可用缩放比例，避免原图边界小于最小比例时产生反向范围
fn scale(raw: f32, maximum: f32, minimum: f32) -> f32 {
    let maximum = maximum.max(0.0);
    raw.clamp(minimum.min(maximum), maximum)
}

/// 【普通标注】【角点矩形】按 anchor、sign、size 和 scale 返回角点固定的归一化矩形，退化时返回空值
fn corner_rect(anchor: Point, sign: Point, size: Point, scale: f32) -> Option<Rect> {
    let end = (
        anchor.0 + sign.0 * size.0 * scale,
        anchor.1 + sign.1 * size.1 * scale,
    );
    Rect::from_ltrb(
        anchor.0.min(end.0),
        anchor.1.min(end.1),
        anchor.0.max(end.0),
        anchor.1.max(end.1),
    )
}

/// 【普通标注】【等比角点】按 before、pointer、canvas、minimum、anchor 和 sign 返回固定角点的等比缩放矩形
fn aspect_corner(
    before: Rect,
    pointer: Point,
    canvas: Rect,
    minimum: f32,
    anchor: Point,
    sign: Point,
) -> Option<Rect> {
    let raw = ((pointer.0 - anchor.0).abs() / before.width())
        .max((pointer.1 - anchor.1).abs() / before.height());
    let available_x = if sign.0 > 0.0 {
        canvas.right() - anchor.0
    } else {
        anchor.0 - canvas.left()
    };
    let available_y = if sign.1 > 0.0 {
        canvas.bottom() - anchor.1
    } else {
        anchor.1 - canvas.top()
    };
    let maximum = (available_x / before.width()).min(available_y / before.height());
    corner_rect(
        anchor,
        sign,
        (before.width(), before.height()),
        scale(raw, maximum, minimum),
    )
}

/// 【普通标注】【水平等比边】按 before、pointer、canvas、minimum、anchor_x 和 sign 返回以对边为轴的等比矩形
fn aspect_horizontal(
    before: Rect,
    pointer: Point,
    canvas: Rect,
    minimum: f32,
    anchor_x: f32,
    sign: f32,
) -> Option<Rect> {
    let center_y = center(before).1;
    let raw = (pointer.0 - anchor_x).abs() / before.width();
    let available_x = if sign > 0.0 {
        canvas.right() - anchor_x
    } else {
        anchor_x - canvas.left()
    };
    let available_y = 2.0 * (center_y - canvas.top()).min(canvas.bottom() - center_y);
    let maximum = (available_x / before.width()).min(available_y / before.height());
    let scale = scale(raw, maximum, minimum);
    let opposite = anchor_x + sign * before.width() * scale;
    Rect::from_ltrb(
        anchor_x.min(opposite),
        center_y - before.height() * scale * 0.5,
        anchor_x.max(opposite),
        center_y + before.height() * scale * 0.5,
    )
}

/// 【普通标注】【垂直等比边】按 before、pointer、canvas、minimum、anchor_y 和 sign 返回以对边为轴的等比矩形
fn aspect_vertical(
    before: Rect,
    pointer: Point,
    canvas: Rect,
    minimum: f32,
    anchor_y: f32,
    sign: f32,
) -> Option<Rect> {
    let center_x = center(before).0;
    let raw = (pointer.1 - anchor_y).abs() / before.height();
    let available_y = if sign > 0.0 {
        canvas.bottom() - anchor_y
    } else {
        anchor_y - canvas.top()
    };
    let available_x = 2.0 * (center_x - canvas.left()).min(canvas.right() - center_x);
    let maximum = (available_x / before.width()).min(available_y / before.height());
    let scale = scale(raw, maximum, minimum);
    let opposite = anchor_y + sign * before.height() * scale;
    Rect::from_ltrb(
        center_x - before.width() * scale * 0.5,
        anchor_y.min(opposite),
        center_x + before.width() * scale * 0.5,
        anchor_y.max(opposite),
    )
}

/// 【普通标注】【八向缩放】按 before、handle、pointer、canvas、pixel_scale 与 keep_aspect 返回原版缩放范围，非缩放控制点返回空值
pub(crate) fn resized(
    before: Rect,
    handle: SelectionHandle,
    pointer: Point,
    canvas: Rect,
    pixel_scale: f32,
    keep_aspect: bool,
) -> Option<Rect> {
    use SelectionHandle::{Bottom, BottomLeft, BottomRight, Left, Right, Top, TopLeft, TopRight};
    let pixel = 1.0 / pixel_scale.max(0.001);
    let minimum = 8.0 * pixel;
    let pointer = (
        pointer
            .0
            .clamp(canvas.left(), (canvas.right() - pixel).max(canvas.left())),
        pointer
            .1
            .clamp(canvas.top(), (canvas.bottom() - pixel).max(canvas.top())),
    );
    // 1. 【普通标注】【比例约束】角点固定对角，边中点固定对边并围绕另一方向中心伸缩
    if keep_aspect && before.width() > 0.0 && before.height() > 0.0 {
        let minimum = (minimum / before.width()).max(minimum / before.height());
        return match handle {
            TopLeft => aspect_corner(
                before,
                pointer,
                canvas,
                minimum,
                (before.right(), before.bottom()),
                (-1.0, -1.0),
            ),
            TopRight => aspect_corner(
                before,
                pointer,
                canvas,
                minimum,
                (before.left(), before.bottom()),
                (1.0, -1.0),
            ),
            BottomLeft => aspect_corner(
                before,
                pointer,
                canvas,
                minimum,
                (before.right(), before.top()),
                (-1.0, 1.0),
            ),
            BottomRight => aspect_corner(
                before,
                pointer,
                canvas,
                minimum,
                (before.left(), before.top()),
                (1.0, 1.0),
            ),
            Left => aspect_horizontal(before, pointer, canvas, minimum, before.right(), -1.0),
            Right => aspect_horizontal(before, pointer, canvas, minimum, before.left(), 1.0),
            Top => aspect_vertical(before, pointer, canvas, minimum, before.bottom(), -1.0),
            Bottom => aspect_vertical(before, pointer, canvas, minimum, before.top(), 1.0),
            _ => None,
        };
    }
    let (mut left, mut top, mut right, mut bottom) =
        (before.left(), before.top(), before.right(), before.bottom());
    // 2. 【普通标注】【独立边界】分别限制被拖动的边，原图像素最小尺寸不能因跨越对边变成翻转
    if matches!(handle, Left | TopLeft | BottomLeft) {
        left = pointer
            .0
            .clamp(canvas.left(), (right - minimum).max(canvas.left()));
    }
    if matches!(handle, Right | TopRight | BottomRight) {
        right = pointer
            .0
            .clamp((left + minimum).min(canvas.right()), canvas.right());
    }
    if matches!(handle, Top | TopLeft | TopRight) {
        top = pointer
            .1
            .clamp(canvas.top(), (bottom - minimum).max(canvas.top()));
    }
    if matches!(handle, Bottom | BottomLeft | BottomRight) {
        bottom = pointer
            .1
            .clamp((top + minimum).min(canvas.bottom()), canvas.bottom());
    }
    if !matches!(
        handle,
        Left | Right | Top | Bottom | TopLeft | TopRight | BottomLeft | BottomRight
    ) {
        return None;
    }
    Rect::from_ltrb(left, top, right, bottom)
}
