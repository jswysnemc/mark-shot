//! 【选择面板】【网格绘制】缓存与副工具栏同源的圆角浮层内容，绘制标题、柔和网格单元、选中态与底部说明

use super::ChoicePopover;
use crate::{
    renderer::{Painter, paths::rounded_rect_path, text::TextStyle},
    theme::{font, toolbar::Palette},
    ui::{language::Language, panel::UiPanel},
};
use cosmic_text::Weight;
use tiny_skia::{Color, Rect, Stroke};

const POPOVER_RADIUS: f32 = 10.5;
const TILE_RADIUS: f32 = 6.5;

/// 【选择面板】【标题本地化】按当前界面语言转换子面板标题，返回显示文本
fn localized_title(title: &'static str) -> &'static str {
    if Language::current() == Language::Chinese {
        match title {
            "Filter effect" => "打码与滤镜",
            "Arrow style" => "箭头样式",
            "Line style" => "线条样式",
            "Marker shape" => "标记形状",
            "Ruler mode" => "测量模式",
            "Number style" => "序号样式",
            _ => title,
        }
    } else {
        title
    }
}

/// 【选择面板】【细边绘制】在 painter 中为 rect 绘制指定半径与颜色的一像素圆角细边，无返回值
fn stroke_rounded(painter: &mut Painter<'_>, rect: Rect, radius: f32, color: Color) {
    if let Some(path) = rounded_rect_path(&rect, radius, true, true, true, true) {
        painter.stroke_path(
            &path,
            color,
            &Stroke {
                width: 1.0,
                ..Default::default()
            },
        );
    }
}

/// 【选择面板】【面板绘制】把 panel 绘入 painter；draw_item 根据索引、矩形和主题颜色绘制条目，无返回值
pub fn draw_choice_popover(
    painter: &mut Painter<'_>,
    panel: &mut ChoicePopover,
    mut draw_item: impl FnMut(&mut Painter<'_>, usize, Rect, Color),
) {
    let Some(rect) = panel.rect() else {
        return;
    };
    let title_text = localized_title(panel.title);
    let footer_text = panel
        .hovered
        .or(panel.active)
        .and_then(|index| panel.options.get(index))
        .map(|opt| Language::current().text(opt.label));
    if Language::current() == Language::Chinese {
        painter.ensure_ui_text(title_text);
        if let Some(footer) = footer_text {
            painter.ensure_ui_text(footer);
        }
    }

    let mut cache = panel.pixmap.take();
    let look = panel.look();
    painter.panel_unframed(
        &mut cache,
        (rect.left(), rect.top()),
        panel.layout.size,
        panel.opacity,
        panel.drawn_look != Some(look),
        |painter| {
            let layout = panel.layout;
            let palette = Palette::current();
            // 1. 绘制与副工具栏同源的圆角浮层卡片与一像素外描边
            if let Some(frame) = Rect::from_xywh(
                0.5,
                0.5,
                (layout.size.0 - 1.0).max(1.0),
                (layout.size.1 - 1.0).max(1.0),
            ) {
                painter.fill_rounded(frame, POPOVER_RADIUS, palette.popup_fill);
                stroke_rounded(painter, frame, POPOVER_RADIUS, palette.popup_border.color());
            }

            if let Some(title) = Rect::from_xywh(
                layout.padding,
                layout.padding,
                layout.size.0 - layout.padding * 2.0,
                layout.title_height,
            ) {
                painter.text(
                    title,
                    title_text,
                    TextStyle::new(font::small(), palette.muted_text.color())
                        .weight(Weight::MEDIUM),
                );
            }

            // 2. 绘制网格选项单元，区分选中高亮态、柔和悬停态与默认轻底座
            for index in 0..panel.options.len() {
                let Some(cell) = layout.item_rect(index, panel.first_row) else {
                    continue;
                };
                let selected = panel.active == Some(index);
                let hovered = panel.hovered == Some(index);
                let tint = if selected {
                    painter.fill_rounded(cell, TILE_RADIUS, palette.active_fill);
                    stroke_rounded(painter, cell, TILE_RADIUS, palette.active_border.color());
                    palette.strong_text.color()
                } else if hovered {
                    painter.fill_rounded(cell, TILE_RADIUS, palette.soft_hover_fill);
                    stroke_rounded(
                        painter,
                        cell,
                        TILE_RADIUS,
                        palette.soft_hover_border.color(),
                    );
                    palette.strong_text.color()
                } else {
                    painter.fill_rounded(
                        cell,
                        TILE_RADIUS,
                        palette
                            .control_fill
                            .with_alpha(palette.control_fill.alpha() / 2 + 4),
                    );
                    palette.text.color()
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

            // 3. 在底部绘制当前悬停或选中项的紧凑说明文字
            if let Some(footer) = footer_text
                && let Some(label) = Rect::from_xywh(
                    layout.padding,
                    layout.size.1 - layout.padding - layout.label_height,
                    layout.size.0 - layout.padding * 2.0,
                    layout.label_height,
                )
            {
                painter.text(
                    label,
                    footer,
                    TextStyle::new(font::small(), palette.strong_text.color())
                        .weight(Weight::MEDIUM),
                );
            }
        },
    );
    panel.pixmap = cache;
    panel.drawn_look = Some(look);
}
