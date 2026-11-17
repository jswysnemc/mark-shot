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

/// 【属性取色】【细边颜色】按当前明暗主题返回控件内边框颜色，保证浅色面板上白色与浅色区域边界清晰
pub(super) fn hairline_color() -> Rgba {
    if crate::theme::Mode::current() == crate::theme::Mode::Dark {
        Rgba(255, 255, 255, 42)
    } else {
        Rgba(15, 23, 42, 34)
    }
}

/// 【属性取色】【透明棋盘】按 rect 与 mask 绘制主题自适应四像素棋盘，浅色与深色模式均保持柔和对比
pub(super) fn checkerboard(p: &mut Painter, rect: Rect, mask: Option<&Mask>) {
    let transform = p.transform();
    let (light_cell, dark_cell) = if crate::theme::Mode::current() == crate::theme::Mode::Dark {
        (Rgba(70, 76, 86, 255), Rgba(45, 50, 58, 255))
    } else {
        (Rgba(241, 245, 249, 255), Rgba(203, 213, 225, 255))
    };
    let mut paint = Paint::default();
    paint.set_color(light_cell.color());
    p.canvas().fill_rect(rect, &paint, transform, mask);
    paint.set_color(dark_cell.color());
    let cell_size = 4.0_f32;
    let mut row = 0;
    let mut y = rect.top();
    while y < rect.bottom() {
        let mut x = rect.left() + if row % 2 == 0 { cell_size } else { 0.0 };
        while x < rect.right() {
            if let Some(cell) = Rect::from_xywh(
                x,
                y,
                cell_size.min(rect.right() - x),
                cell_size.min(rect.bottom() - y),
            ) {
                p.canvas().fill_rect(cell, &paint, transform, mask);
            }
            x += cell_size * 2.0;
        }
        row += 1;
        y += cell_size;
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
    border_width(p, rect, radius, color, 1.0);
}

/// 【属性取色】【可调线宽描边】按 width 对 rect 绘制 radius 圆角边框，供默认态与悬停态复用
pub(super) fn border_width(p: &mut Painter, rect: Rect, radius: f32, color: Rgba, width: f32) {
    if let Some(path) = path(rect, radius) {
        p.stroke_path(
            &path,
            color,
            &Stroke {
                width,
                ..Default::default()
            },
        );
    }
}

/// 【属性取色】【圆形握柄】按指定外环半径 outer_radius、内核半径 inner_radius 与描边宽度在 bounds 内绘制取色握柄
fn handle_ring(
    p: &mut Painter,
    center: (f32, f32),
    fill: Color,
    bounds: Rect,
    outer_radius: f32,
    inner_radius: f32,
    stroke_width: f32,
) {
    let clip = mask(p, bounds, 0.0);
    let transform = p.transform();
    let mut paint = Paint {
        anti_alias: true,
        ..Default::default()
    };
    paint.set_color(Rgba(255, 255, 255, 246).color());
    let mut builder = tiny_skia::PathBuilder::new();
    builder.push_circle(center.0, center.1, outer_radius);
    if let Some(path) = builder.finish() {
        p.canvas()
            .fill_path(&path, &paint, FillRule::Winding, transform, clip.as_ref());
        paint.set_color(Rgba(15, 23, 42, 196).color());
        p.canvas().stroke_path(
            &path,
            &paint,
            &Stroke {
                width: stroke_width,
                ..Default::default()
            },
            transform,
            clip.as_ref(),
        );
    }
    let mut builder = tiny_skia::PathBuilder::new();
    builder.push_circle(center.0, center.1, inner_radius);
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
    let clip = mask(p, inner, 5.0);
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
    border(p, inner, 5.0, hairline_color());
    handle_ring(
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
        6.0,
        3.5,
        1.5,
    );
}

/// 【属性取色】【横向轨道】绘制 picker 的色相与 alpha，端点握柄与轨道圆弧同心且保持完整圆形不裁切
pub(super) fn draw(p: &mut Painter, picker: &mut ColorPickerPopover) {
    // 1. 先绘制二维明度饱和度场，再绘制横向色相与透明度胶囊轨道
    sv(p, picker);
    for (rect, alpha) in [
        (layout::hue((0.0, 0.0)), false),
        (layout::alpha((0.0, 0.0)), true),
    ] {
        let inner = Rect::from_xywh(
            rect.left() + 0.5,
            rect.top() + 1.0,
            rect.width() - 1.0,
            rect.height() - 2.0,
        )
        .unwrap();
        let radius = inner.height() / 2.0;
        let clip = mask(p, inner, radius);
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
        border(p, inner, radius, hairline_color());
        // 2. 握柄圆心沿胶囊两端圆心范围线性插值，保证零值与满值时圆环完整不被左右边界截断
        let cap_pad = rect.height() / 2.0;
        let travel = (rect.width() - cap_pad * 2.0).max(0.0);
        handle_ring(
            p,
            (
                rect.left() + cap_pad + fraction.clamp(0.0, 1.0) * travel,
                rect.top() + rect.height() / 2.0,
            ),
            fill,
            rect,
            5.0,
            2.8,
            1.25,
        );
    }
}
