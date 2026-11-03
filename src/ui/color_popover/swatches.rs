//! 【属性取色】【颜色样本】当前预览与七项历史复用原版方块、透明棋盘和白色细边

use super::{ColorPickerPopover, layout, tracks};
use crate::{renderer::Painter, theme::Rgba};
use tiny_skia::{Color, Paint, Rect};

/// 【属性取色】【方块预览】在 rect 内绘制 color；半透明颜色不叠加选择态或悬停尺寸
fn swatch(p: &mut Painter, rect: Rect, color: Color) {
    let inner = Rect::from_xywh(
        rect.left() + 0.5,
        rect.top() + 0.5,
        rect.width() - 1.0,
        rect.height() - 1.0,
    )
    .unwrap();
    let mask = tracks::mask(p, inner, 6.0);
    tracks::checkerboard(p, inner, mask.as_ref());
    let mut paint = Paint::default();
    paint.set_color(color);
    let transform = p.transform();
    p.canvas()
        .fill_rect(inner, &paint, transform, mask.as_ref());
    tracks::border(p, inner, 6.0, Rgba(255, 255, 255, 56));
}

/// 【属性取色】【历史绘制】按 picker 当前状态绘制八个槽位；空槽透明但仍显示棋盘，与 Qt 禁用控件一致
pub(super) fn draw(p: &mut Painter, picker: &ColorPickerPopover) {
    swatch(p, layout::current((0.0, 0.0)), picker.sv_square.color());
    for index in 0..8 {
        let rect = layout::history((0.0, 0.0), index).unwrap();
        swatch(
            p,
            rect,
            picker
                .palette()
                .get(index)
                .copied()
                .unwrap_or(Color::TRANSPARENT),
        );
    }
}
