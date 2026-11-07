//! 【字体弹层】【滚动条绘制】轨道、手柄与箭头复用界面主题，不改变原版十四像素几何

use super::{FontPanel, draw::inset, state::Hit};
use crate::{
    renderer::Painter,
    theme::{Rgba, toolbar::Palette},
    ui::text_properties::draw::border,
};
use tiny_skia::Rect;

/// 【字体弹层】【滚动条内容】按 panel 的滚动与鼠标状态，以 palette 绘入 p，无返回值
pub(super) fn draw(p: &mut Painter, panel: &FontPanel, palette: Palette) {
    let Some(thumb) = panel.thumb() else { return };
    let list = panel.layout.list;
    let track = Rect::from_xywh(list.right() - 14.0, list.top(), 14.0, list.height()).unwrap();
    p.fill_rounded(track, 0.0, palette.list_fill);
    let held = panel.scrollbar_drag.is_some() || panel.pressed == Some(Hit::Scrollbar);
    let hovered = panel.hovered == Some(Hit::Scrollbar);
    p.fill_rounded(
        thumb,
        2.0,
        if held {
            palette.pressed_fill
        } else if hovered {
            palette.hover_fill
        } else {
            palette.muted_text
        },
    );
    border(p, inset(thumb, 0.5), 2.0, 1.0, palette.control_border);
    for (y, up, hit) in [
        (list.top(), true, Hit::ScrollUp),
        (list.bottom() - 14.0, false, Hit::ScrollDown),
    ] {
        let cell = Rect::from_xywh(list.right() - 14.0, y, 14.0, 14.0).unwrap();
        let active = panel.pressed == Some(hit) || panel.hovered == Some(hit);
        let fill = if panel.pressed == Some(hit) {
            palette.pressed_fill
        } else if active {
            palette.hover_fill
        } else {
            palette.control_fill
        };
        p.fill_rounded(cell, 1.0, fill);
        border(p, inset(cell, 0.5), 1.0, 1.0, palette.control_border);
        arrow(
            p,
            (cell.left() + 7.0, cell.top() + 7.0),
            4.0,
            up,
            if active {
                palette.hover_text
            } else {
                palette.text
            },
        );
    }
}

/// 【字体弹层】【箭头绘制】按 center、radius 和 up 方向绘制 ink 三角形至 p，无返回值
pub(super) fn arrow(p: &mut Painter, center: (f32, f32), radius: f32, up: bool, ink: Rgba) {
    let direction = if up { 1.0 } else { -1.0 };
    let mut path = tiny_skia::PathBuilder::new();
    path.move_to(center.0 - radius, center.1 + direction * 2.0);
    path.line_to(center.0 + radius, center.1 + direction * 2.0);
    path.line_to(center.0, center.1 - direction * (radius - 1.0));
    path.close();
    if let Some(path) = path.finish() {
        p.fill_path(&path, ink);
    }
}
