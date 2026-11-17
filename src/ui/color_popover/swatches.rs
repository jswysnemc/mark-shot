//! 【属性取色】【颜色样本】当前预览与八项等距历史色块，支持不透明色防杂边、空槽占位与悬停反馈

use super::{ColorPickerPopover, ColorPopoverElement, layout, tracks};
use crate::{renderer::Painter, theme::toolbar::Palette};
use tiny_skia::{Color, Paint, Rect};

/// 【属性取色】【方块预览】按 color、radius 与 hovered 在 rect 内绘制色块；不透明色跳过棋盘避免圆角暗色杂边
fn swatch(p: &mut Painter, rect: Rect, color: Option<Color>, radius: f32, hovered: bool) {
    let inner = Rect::from_xywh(
        rect.left() + 0.5,
        rect.top() + 0.5,
        rect.width() - 1.0,
        rect.height() - 1.0,
    )
    .unwrap();
    let palette = Palette::current();
    let Some(color) = color else {
        p.fill_rounded(inner, radius, palette.control_fill);
        tracks::border(p, inner, radius, palette.control_border);
        return;
    };
    let mask = tracks::mask(p, inner, radius);
    if color.alpha() < 1.0 - f32::EPSILON {
        tracks::checkerboard(p, inner, mask.as_ref());
    }
    let mut paint = Paint::default();
    paint.set_color(color);
    let transform = p.transform();
    p.canvas()
        .fill_rect(inner, &paint, transform, mask.as_ref());
    if hovered {
        tracks::border_width(p, inner, radius, palette.hover_border, 1.5);
    } else {
        tracks::border(p, inner, radius, tracks::hairline_color());
    }
}

/// 【属性取色】【历史绘制】绘制左侧当前预览及底部八个无重叠槽位，可点击历史项提供悬停边框高亮
pub(super) fn draw(p: &mut Painter, picker: &ColorPickerPopover) {
    swatch(
        p,
        layout::current((0.0, 0.0)),
        Some(picker.sv_square.color()),
        6.0,
        false,
    );
    for index in 0..8 {
        let rect = layout::history((0.0, 0.0), index).unwrap();
        let entry = picker.palette().get(index).copied();
        let hovered = index >= 1
            && entry.is_some()
            && picker.hovered == Some(ColorPopoverElement::Swatch(index));
        swatch(p, rect, entry, 5.0, hovered);
    }
}
