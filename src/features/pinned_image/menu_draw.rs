//! 【贴图】【菜单内容】主菜单和语言子菜单共用配色、行高、勾选与细边，绘制不执行动作

#[cfg(test)]
mod tests;

use crate::{
    features::pinned_image::menu,
    renderer::{Painter, text::TextStyle},
};
use tiny_skia::{Pixmap, Rect, Stroke};

pub(crate) struct Row {
    pub label: String,
    pub enabled: bool,
    pub checked: bool,
    pub danger: bool,
    pub submenu: bool,
}

/// 【贴图】【独立菜单绘制】按 rows、hovered、size 和 scale 绘入 canvas，fonts、cache、icons 提供当前组件资源，不创建协议对象
pub(crate) fn draw(
    canvas: &mut Pixmap,
    rows: &[Row],
    hovered: Option<usize>,
    size: (u32, u32),
    scale: f32,
    fonts: &mut cosmic_text::FontSystem,
    cache: &mut cosmic_text::SwashCache,
    icons: &crate::renderer::Icons,
) {
    // 1. 【贴图】【菜单字形】主菜单和语言子菜单独立补充实际缺字，不依赖主编辑器后台字库
    let text = rows
        .iter()
        .map(|row| row.label.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    crate::fonts::ui_fallback::ensure_text(fonts, &text);
    let style = crate::ui::menu_style::MenuStyle::current();
    let mut painter = Painter::new(canvas)
        .with_scale(scale)
        .with_icons(icons)
        .with_text(fonts, cache);
    let bounds = Rect::from_xywh(0.0, 0.0, size.0 as f32, size.1 as f32).unwrap();
    crate::ui::menu_style::draw_popup_frame(
        &mut painter,
        bounds,
        crate::ui::menu_style::POPUP_RADIUS,
    );
    let mut top = menu::PADDING as f32;
    for (index, row) in rows.iter().enumerate() {
        if row.label.is_empty() {
            painter.fill_rounded(
                Rect::from_xywh(8.0, top + 4.5, size.0 as f32 - 16.0, 1.0).unwrap(),
                0.0,
                style.separator,
            );
            top += menu::SEPARATOR as f32;
            continue;
        }
        let active = hovered == Some(index) && row.enabled;
        let rect = Rect::from_xywh(
            menu::PADDING as f32,
            top,
            (size.0 - menu::PADDING * 2) as f32,
            menu::ROW as f32,
        )
        .unwrap();
        let pill_ink = crate::ui::menu_style::draw_row_pill(
            &mut painter,
            rect,
            1.5,
            crate::ui::menu_style::ITEM_RADIUS,
            row.checked && row.enabled,
            active,
            row.danger && row.enabled,
        );
        let ink = if !row.enabled {
            style.disabled_text
        } else {
            pill_ink
        };
        if row.checked {
            painter.icon(crate::ui::icons::CHECK, 14.0, 12.0, top + 7.0, style.accent);
        }
        painter.text(
            Rect::from_xywh(34.0, top, size.0 as f32 - 52.0, menu::ROW as f32).unwrap(),
            &row.label,
            TextStyle::new(12.0, ink),
        );
        if row.submenu {
            let mut path = tiny_skia::PathBuilder::new();
            let (x, y) = (size.0 as f32 - 15.0, top + menu::ROW as f32 / 2.0);
            path.move_to(x - 2.0, y - 3.5);
            path.line_to(x + 2.0, y);
            path.line_to(x - 2.0, y + 3.5);
            if let Some(path) = path.finish() {
                painter.stroke_path(
                    &path,
                    ink,
                    &Stroke {
                        width: 1.5,
                        line_cap: tiny_skia::LineCap::Round,
                        line_join: tiny_skia::LineJoin::Round,
                        ..Default::default()
                    },
                );
            }
        }
        top += menu::ROW as f32;
    }
}
