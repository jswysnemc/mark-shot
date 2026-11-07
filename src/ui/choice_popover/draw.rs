//! 【选择面板】【网格绘制】缓存紧凑面板内容，绘制标题、图标、选中状态与标签

use tiny_skia::{Color, Rect};

use super::ChoicePopover;
use crate::renderer::Painter;
use crate::renderer::text::TextStyle;
use crate::theme::{font, radius};
use crate::ui::panel::UiPanel;

/// 【选择面板】【面板绘制】把 panel 绘入 painter；draw_item 根据索引、矩形和主题颜色绘制条目，无返回值
pub fn draw_choice_popover(
    painter: &mut Painter<'_>,
    panel: &mut ChoicePopover,
    mut draw_item: impl FnMut(&mut Painter<'_>, usize, Rect, Color),
) {
    let Some(rect) = panel.rect() else {
        return;
    };
    let mut cache = panel.pixmap.take();
    let look = panel.look();
    painter.panel(
        &mut cache,
        (rect.left(), rect.top()),
        panel.layout.size,
        panel.opacity,
        panel.drawn_look != Some(look),
        |painter| {
            let layout = panel.layout;
            let theme = crate::theme::color::theme();
            if let Some(rect) = Rect::from_xywh(0.0, 0.0, layout.size.0, layout.size.1) {
                painter.fill_rounded(rect, radius::panel(), theme.background.color());
            }
            if let Some(title) = Rect::from_xywh(
                layout.padding,
                layout.padding,
                layout.size.0 - layout.padding * 2.0,
                layout.title_height,
            ) {
                painter.text(
                    title,
                    panel.title,
                    TextStyle::new(font::label(), theme.foreground.color()),
                );
            }
            for index in 0..panel.options.len() {
                let Some(cell) = layout.item_rect(index, panel.first_row) else {
                    continue;
                };
                let selected = panel.active == Some(index);
                let hovered = panel.hovered == Some(index);
                let tint = if selected || hovered {
                    painter.fill_rounded(
                        cell,
                        radius::item(),
                        if selected {
                            theme.accent.color()
                        } else {
                            theme.hover.color()
                        },
                    );
                    if hovered && !selected {
                        crate::theme::toolbar::Palette::current().hover_icon.color()
                    } else {
                        theme.on_accent.color()
                    }
                } else {
                    theme.foreground.color()
                };
                let inset = (layout.cell_size * 0.17).max(1.0);
                if let Some(icon) = Rect::from_xywh(
                    cell.left() + inset,
                    cell.top() + inset,
                    cell.width() - inset * 2.0,
                    cell.height() - inset * 2.0,
                ) {
                    draw_item(painter, index, icon, tint);
                }
            }
            if let Some(option) = panel
                .hovered
                .or(panel.active)
                .and_then(|index| panel.options.get(index))
                && let Some(label) = Rect::from_xywh(
                    layout.padding,
                    layout.size.1 - layout.padding - layout.label_height,
                    layout.size.0 - layout.padding * 2.0,
                    layout.label_height,
                )
            {
                painter.text(
                    label,
                    option.label,
                    TextStyle::new(font::small(), theme.muted.color()),
                );
            }
        },
    );
    panel.pixmap = cache;
    panel.drawn_look = Some(look);
}
