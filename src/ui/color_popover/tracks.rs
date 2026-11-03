//! 【属性取色】【轨道绘制】矩形 HSV、横向色相及透明棋盘轨道共用原版圆角与握柄

use super::{ColorPickerPopover, hsv_to_color, layout};
use crate::{renderer::Painter, theme::Rgba};
use tiny_skia::{
    Color, FillRule, GradientStop, LinearGradient, Mask, Paint, Point, Rect, SpreadMode, Stroke,
    Transform,
};

/// 【属性取色】【圆角路径】由 rect、radius 构造路径，返回有效路径或空值
fn path(rect: Rect, radius: f32) -> Option<tiny_skia::Path> {
    crate::renderer::paths::rounded_rect_path(&rect, radius, true, true, true, true)
}

/// 【属性取色】【渐变填充】以 stops 在 rect 内绘制渐变；horizontal 选择横向，mask 限制控件边界
fn gradient(
    p: &mut Painter,
    rect: Rect,
    stops: Vec<GradientStop>,
    horizontal: bool,
    mask: Option<&Mask>,
) {
    let end = if horizontal {
        Point::from_xy(rect.right(), rect.top())
    } else {
        Point::from_xy(rect.left(), rect.bottom())
    };
    let Some(shader) = LinearGradient::new(
        Point::from_xy(rect.left(), rect.top()),
        end,
        stops,
        SpreadMode::Pad,
        Transform::identity(),
    ) else {
        return;
    };
    let paint = Paint {
        shader,
        ..Default::default()
    };
    let transform = p.transform();
    p.canvas().fill_rect(rect, &paint, transform, mask);
}

/// 【属性取色】【透明棋盘】按 rect 与 mask 绘制五像素棋盘，尺寸和颜色与 Qt 取色器一致
pub(super) fn checkerboard(p: &mut Painter, rect: Rect, mask: Option<&Mask>) {
    let transform = p.transform();
    let mut paint = Paint::default();
    paint.set_color(Rgba(70, 76, 86, 255).color());
    p.canvas().fill_rect(rect, &paint, transform, mask);
    paint.set_color(Rgba(45, 50, 58, 255).color());
    let mut row = 0;
    let mut y = rect.top();
    while y < rect.bottom() {
        let mut x = rect.left() + if row % 2 == 0 { 5.0 } else { 0.0 };
        while x < rect.right() {
            if let Some(cell) = Rect::from_xywh(
                x,
                y,
                5.0_f32.min(rect.right() - x),
                5.0_f32.min(rect.bottom() - y),
            ) {
                p.canvas().fill_rect(cell, &paint, transform, mask);
            }
            x += 10.0;
        }
        row += 1;
        y += 5.0;
    }
}

/// 【属性取色】【控件裁切】按 p 的物理缓冲创建 rect 圆角 mask；返回缓存可用的透明度遮罩
pub(super) fn mask(p: &mut Painter, rect: Rect, radius: f32) -> Option<Mask> {
    let transform = p.transform();
    let canvas = p.canvas();
    let mut mask = Mask::new(canvas.width(), canvas.height())?;
    mask.fill_path(&path(rect, radius)?, FillRule::Winding, true, transform);
    Some(mask)
}

/// 【属性取色】【细边描绘】对 rect 绘制 radius 圆角及 color 单像素边，不使用旧主题双色边
pub(super) fn border(p: &mut Painter, rect: Rect, radius: f32, color: Rgba) {
    if let Some(path) = path(rect, radius) {
        p.stroke_path(
            &path,
            color,
            &Stroke {
                width: 1.0,
                ..Default::default()
            },
        );
    }
}

/// 【属性取色】【握柄绘制】在 center 绘制 fill 内核、白底与深色边；bounds 按 Qt 独立控件边界裁切握柄
fn handle(p: &mut Painter, center: (f32, f32), fill: Color, bounds: Rect) {
    let clip = mask(p, bounds, 0.0);
    let transform = p.transform();
    let mut paint = Paint {
        anti_alias: true,
        ..Default::default()
    };
    paint.set_color(Rgba(255, 255, 255, 230).color());
    let mut builder = tiny_skia::PathBuilder::new();
    builder.push_circle(center.0, center.1, 6.0);
    if let Some(path) = builder.finish() {
        p.canvas()
            .fill_path(&path, &paint, FillRule::Winding, transform, clip.as_ref());
        paint.set_color(Rgba(15, 17, 23, 220).color());
        p.canvas().stroke_path(
            &path,
            &paint,
            &Stroke {
                width: 1.5,
                ..Default::default()
            },
            transform,
            clip.as_ref(),
        );
    }
    let mut builder = tiny_skia::PathBuilder::new();
    builder.push_circle(center.0, center.1, 3.5);
    if let Some(path) = builder.finish() {
        paint.set_color(fill);
        p.canvas()
            .fill_path(&path, &paint, FillRule::Winding, transform, clip.as_ref());
    }
}

/// 【属性取色】【明度绘制】按 picker 当前 hue 绘制 184×112 色阶，不复制截图像素
fn sv(p: &mut Painter, picker: &mut ColorPickerPopover) {
    let rect = layout::sv((0.0, 0.0));
    let inner = Rect::from_xywh(
        rect.left() + 0.5,
        rect.top() + 0.5,
        rect.width() - 1.0,
        rect.height() - 1.0,
    )
    .unwrap();
    let clip = mask(p, inner, 4.0);
    let transform = p.transform();
    let mut paint = Paint::default();
    paint.set_color(hsv_to_color(picker.sv_square.hue, 1.0, 1.0));
    p.canvas()
        .fill_rect(inner, &paint, transform, clip.as_ref());
    gradient(
        p,
        inner,
        vec![
            GradientStop::new(0.0, Color::WHITE),
            GradientStop::new(1.0, Color::from_rgba8(255, 255, 255, 0)),
        ],
        true,
        clip.as_ref(),
    );
    gradient(
        p,
        inner,
        vec![
            GradientStop::new(0.0, Color::from_rgba8(0, 0, 0, 0)),
            GradientStop::new(1.0, Color::BLACK),
        ],
        false,
        clip.as_ref(),
    );
    border(p, inner, 4.0, Rgba(255, 255, 255, 28));
    handle(
        p,
        (
            inner.left() + picker.sv_square.sv.0 * inner.width(),
            inner.top() + (1.0 - picker.sv_square.sv.1) * inner.height(),
        ),
        hsv_to_color(
            picker.sv_square.hue,
            picker.sv_square.sv.0,
            picker.sv_square.sv.1,
        ),
        rect,
    );
}

/// 【属性取色】【横向轨道】绘制 picker 的色相与 alpha，取样原点及握柄位置与 Qt 一致
pub(super) fn draw(p: &mut Painter, picker: &mut ColorPickerPopover) {
    // 1. 【属性取色】【色阶区域】先绘制二维场，再绘制独立色相和透明度轨道
    sv(p, picker);
    for (rect, alpha) in [
        (layout::hue((0.0, 0.0)), false),
        (layout::alpha((0.0, 0.0)), true),
    ] {
        let inner = Rect::from_xywh(
            rect.left() + 1.0,
            rect.top() + 1.5,
            rect.width() - 2.0,
            rect.height() - 3.0,
        )
        .unwrap();
        let clip = mask(p, inner, inner.height() / 2.0);
        let base = hsv_to_color(
            picker.sv_square.hue,
            picker.sv_square.sv.0,
            picker.sv_square.sv.1,
        );
        let color = base.to_color_u8();
        let (stops, fraction, fill) = if alpha {
            checkerboard(p, inner, clip.as_ref());
            (
                vec![
                    GradientStop::new(
                        0.0,
                        Color::from_rgba8(color.red(), color.green(), color.blue(), 0),
                    ),
                    GradientStop::new(1.0, base),
                ],
                picker.sv_square.alpha as f32 / 255.0,
                Color::from_rgba8(
                    color.red(),
                    color.green(),
                    color.blue(),
                    picker.sv_square.alpha,
                ),
            )
        } else {
            (
                (0..=6)
                    .map(|i| {
                        GradientStop::new(
                            i as f32 / 6.0,
                            hsv_to_color((i * 60 % 360) as f32, 1.0, 1.0),
                        )
                    })
                    .collect(),
                picker.sv_square.hue / 359.0,
                hsv_to_color(picker.sv_square.hue, 1.0, 1.0),
            )
        };
        gradient(p, inner, stops, true, clip.as_ref());
        border(p, inner, inner.height() / 2.0, Rgba(255, 255, 255, 26));
        // 2. 【属性取色】【轨道握柄】显示坐标使用内部轨道，输入坐标仍使用完整控件宽度
        handle(
            p,
            (
                inner.left() + fraction * inner.width(),
                inner.top() + inner.height() / 2.0,
            ),
            fill,
            rect,
        );
    }
}
