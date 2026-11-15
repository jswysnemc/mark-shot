//! 【图片窗口】【窗口按钮】窗口操作沿用工具栏图标、圆角及交互配色

use super::Hit;
use crate::{
    renderer::Painter,
    theme::toolbar::Palette,
    ui::{controls, icons, toolbar::metrics},
};
use tiny_skia::Rect;

/// 【图片窗口】【按钮绘制】按 painter、area 和 control 绘制窗口操作，hovered 与 pressed 控制交互状态；无返回值
pub(super) fn draw(
    painter: &mut Painter<'_>,
    area: Rect,
    control: Hit,
    hovered: bool,
    pressed: bool,
) {
    let palette = Palette::current();
    let danger = control == Hit::Close;
    let (fill, border, ink) = if danger && (hovered || pressed) {
        (
            Some(palette.danger_hover_fill),
            palette.danger_hover_border,
            palette.danger_hover_text,
        )
    } else if pressed {
        (
            Some(palette.pressed_fill),
            palette.pressed_border,
            palette.hover_icon,
        )
    } else if hovered {
        (
            Some(palette.hover_fill),
            palette.hover_border,
            palette.hover_icon,
        )
    } else {
        (
            None,
            palette.panel_border,
            crate::config::get().toolbar.icon.get(),
        )
    };
    if let Some(fill) = fill {
        controls::frame(painter, area, fill, border, 6.0);
    }
    let svg = match control {
        Hit::Minimize => icons::WINDOW_MINIMIZE,
        Hit::Maximize => icons::WINDOW_MAXIMIZE,
        Hit::Close => icons::CLOSE,
        _ => return,
    };
    let size = metrics::icon()
        .min(area.width() - 4.0)
        .min(area.height() - 4.0);
    painter.icon(
        svg,
        size,
        area.left() + (area.width() - size) / 2.0,
        area.top() + (area.height() - size) / 2.0,
        ink,
    );
}
