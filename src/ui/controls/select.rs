//! 【界面控件】【选择框】紧凑选择框只展示当前值，与菜单使用统一颜色和矢量图标

use crate::{
    renderer::{Painter, text::TextStyle},
    theme::window::Palette,
    ui::icons,
};
use tiny_skia::Rect;

/// 【界面控件】【选择框绘制】按 bounds 与 label 绘制当前选项，hovered、expanded 与 enabled 控制交互状态，无返回值
pub fn select(
    painter: &mut Painter<'_>,
    bounds: Rect,
    label: &str,
    hovered: bool,
    expanded: bool,
    enabled: bool,
) {
    let palette = Palette::current();
    super::frame(
        painter,
        bounds,
        palette.base,
        if enabled && (hovered || expanded) {
            palette.accent
        } else {
            palette.border
        },
        6.0,
    );
    let label = super::fit_label(painter, label, 12.0, bounds.width() - 40.0);
    painter.text(
        Rect::from_xywh(
            bounds.left() + 10.0,
            bounds.top(),
            bounds.width() - 36.0,
            bounds.height(),
        )
        .unwrap(),
        &label,
        TextStyle::new(12.0, if enabled { palette.text } else { palette.muted }),
    );
    painter.icon(
        icons::CHEVRON_DOWN,
        14.0,
        bounds.right() - 25.0,
        bounds.top() + (bounds.height() - 14.0) / 2.0,
        if enabled && expanded {
            palette.accent
        } else {
            palette.muted
        },
    );
}
