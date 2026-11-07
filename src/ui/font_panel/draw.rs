//! 【字体弹层】【面板绘制】绘制原版搜索、字体预览列表和字号样式行，可见字体使用独立有界缓存

use super::{
    FontPanel,
    state::{Field, Hit},
};
use crate::{
    renderer::{Painter, text::TextStyle},
    theme::{Rgba, toolbar::Palette},
    ui::{panel::UiPanel, text_properties::draw::border},
};
use tiny_skia::{Pixmap, Rect};

/// 【字体弹层】【缓冲绘制】将 panel 绘入 p，只绘制当前列表页，无返回值
pub fn draw(p: &mut Painter, panel: &mut FontPanel) {
    let Some(rect) = panel.rect() else { return };
    let mut cache = panel.pixmap.take();
    p.panel_unframed(
        &mut cache,
        (rect.left(), rect.top()),
        panel.layout.size,
        1.0,
        true,
        |p| content(p, panel, Palette::current()),
    );
    panel.pixmap = cache;
}

/// 【字体弹层】【内容绘制】按 palette 把 panel 的本地控件与字体预览绘入 p，无返回值
pub(super) fn content(p: &mut Painter, panel: &mut FontPanel, palette: Palette) {
    let layout = panel.layout;
    let frame = Rect::from_xywh(0.5, 0.5, layout.size.0 - 1.0, layout.size.1 - 1.0).unwrap();
    p.fill_rounded(frame, 11.0, palette.popup_fill);
    border(p, frame, 11.0, 1.0, palette.popup_border);
    p.fill_rounded(layout.search, 7.0, palette.control_fill);
    border(
        p,
        inset(layout.search, 0.5),
        7.0,
        1.0,
        if panel.fields.is_editing_key(Field::Search) {
            palette.focus_border
        } else {
            palette.control_border
        },
    );
    let query = panel.query().to_owned();
    let has_text = !query.is_empty()
        || panel
            .fields
            .editing
            .as_ref()
            .is_some_and(|edit| edit.key == Field::Search && edit.field.composition.is_some());
    let search = Rect::from_xywh(
        layout.search.left() + 9.0,
        layout.search.top(),
        layout.search.width() - 36.0,
        layout.search.height(),
    )
    .unwrap();
    let editing = panel
        .fields
        .editing
        .as_ref()
        .filter(|edit| edit.key == Field::Search)
        .map(|edit| &edit.field);
    field(
        p,
        search,
        if !has_text { "Search fonts" } else { &query },
        editing,
        16.0,
        if !has_text {
            palette.muted_text
        } else {
            palette.text
        }
        .color(),
        cosmic_text::Weight::NORMAL,
    );
    if !query.is_empty() {
        p.icon(
            crate::ui::icons::CLOSE,
            14.0,
            layout.search.right() - 21.0,
            layout.search.top() + 7.0,
            palette.muted_text,
        );
    }
    // 1. 【字体弹层】【列表裁切】背景与字形均在独立列表缓冲中绘制，部分行不会覆盖搜索框或字号输入
    let scale = p.scale();
    let rows = panel.rows();
    if let Some(mut image) = Pixmap::new(
        (layout.list.width() * scale).ceil() as u32,
        (layout.list.height() * scale).ceil() as u32,
    ) {
        let mut list = Painter::new(&mut image).with_scale(scale);
        for (index, row) in &rows {
            let selected = panel
                .catalog
                .as_ref()
                .is_some_and(|catalog| catalog.families[*index].name == panel.family);
            let hovered = panel.hovered == Some(Hit::Family(*index));
            let cell = Rect::from_xywh(
                0.5,
                row.top() - layout.list.top() + 1.5,
                row.width() - 1.0,
                row.height() - 3.0,
            )
            .unwrap();
            if selected || hovered {
                list.fill_rounded(
                    cell,
                    7.0,
                    if selected {
                        palette.hover_fill
                    } else {
                        palette.soft_hover_fill
                    },
                );
                border(
                    &mut list,
                    cell,
                    7.0,
                    1.0,
                    if selected {
                        palette.hover_border
                    } else {
                        palette.soft_hover_border
                    },
                );
            }
        }
        if let Some(catalog) = &panel.catalog
            && let Some(text) =
                panel
                    .preview
                    .render(catalog, &rows, layout.list, scale, &panel.family, palette)
        {
            image.draw_pixmap(
                0,
                0,
                text.as_ref(),
                &Default::default(),
                tiny_skia::Transform::identity(),
                None,
            );
        }
        p.draw_pixmap(
            (layout.list.left() * scale).round() as i32,
            (layout.list.top() * scale).round() as i32,
            &image,
        );
    }
    super::scrollbar::draw(p, panel, palette);
    for (index, row) in &rows {
        if panel
            .catalog
            .as_ref()
            .is_some_and(|catalog| catalog.families[*index].name == panel.family)
            && row.top() >= layout.list.top()
            && row.bottom() <= layout.list.bottom()
        {
            p.icon(
                crate::ui::icons::CHECK,
                14.0,
                row.left() + 10.0,
                row.top() + 7.0,
                palette.hover_text,
            );
        }
    }
    // 2. 【字体弹层】【字号样式】底部使用原版二十六像素控件行，不挤占字体列表高度
    for (cell, hit, selected) in [
        (layout.size_field, Hit::Size, false),
        (layout.bold, Hit::Bold, panel.bold),
        (layout.italic, Hit::Italic, panel.italic),
    ] {
        let (fill, outline, _) = control(panel, hit, selected, palette);
        p.fill_rounded(cell, 2.0, fill);
        border(p, inset(cell, 0.5), 2.0, 1.0, outline);
    }
    let value = panel
        .fields
        .editing
        .as_ref()
        .filter(|edit| edit.key == Field::Size)
        .map(|edit| &edit.field);
    let number = value
        .map(|field| field.text.clone())
        .unwrap_or_else(|| format!("{} pt", panel.points.round() as i32));
    let size = Rect::from_xywh(
        layout.size_field.left() + 4.0,
        layout.size_field.top(),
        layout.size_field.width() - 22.0,
        26.0,
    )
    .unwrap();
    field(
        p,
        size,
        &number,
        value,
        16.0,
        palette.text.color(),
        cosmic_text::Weight::NORMAL,
    );
    p.text(
        layout.bold,
        "B",
        TextStyle::new(14.0, control(panel, Hit::Bold, panel.bold, palette).2)
            .center()
            .weight(cosmic_text::Weight::BOLD),
    );
    p.text(
        layout.italic,
        "I",
        TextStyle::new(14.0, control(panel, Hit::Italic, panel.italic, palette).2)
            .center()
            .style(cosmic_text::Style::Italic),
    );
    let arrow_x = layout.size_field.right() - 12.0;
    for (y, up) in [
        (layout.size_field.top() + 8.0, true),
        (layout.size_field.top() + 18.0, false),
    ] {
        super::scrollbar::arrow(p, (arrow_x, y), 3.0, up, palette.muted_text);
    }
}

/// 【字体弹层】【控件配色】按 panel、hit、selected 与 palette 返回底板、边框和文字色，保留鼠标与字段状态
fn control(panel: &FontPanel, hit: Hit, selected: bool, palette: Palette) -> (Rgba, Rgba, Rgba) {
    if panel.pressed == Some(hit) {
        (
            palette.pressed_fill,
            palette.pressed_border,
            palette.hover_text,
        )
    } else if panel.hovered == Some(hit) {
        (palette.hover_fill, palette.hover_border, palette.hover_text)
    } else if selected {
        (palette.active_fill, palette.active_border, palette.text)
    } else {
        let focused = hit == Hit::Size && panel.fields.is_editing_key(Field::Size);
        (
            palette.control_fill,
            if focused {
                palette.focus_border
            } else {
                palette.control_border
            },
            palette.text,
        )
    }
}

/// 【字体弹层】【边框内缩】按 amount 返回 rect 的内缩矩形，调用方保证尺寸有效
pub(super) fn inset(rect: Rect, amount: f32) -> Rect {
    Rect::from_xywh(
        rect.left() + amount,
        rect.top() + amount,
        rect.width() - amount * 2.0,
        rect.height() - amount * 2.0,
    )
    .unwrap()
}

/// 【字体弹层】【字段裁切】把 display 和 editing 按 size、ink、weight 绘入 p 的 rect，光标移动时显示必要前缀，不覆盖相邻控件
fn field(
    p: &mut Painter,
    rect: Rect,
    display: &str,
    editing: Option<&crate::ui::text_field::LineEditState>,
    size: f32,
    ink: tiny_skia::Color,
    weight: cosmic_text::Weight,
) {
    p.line_edit(rect, display, editing, size, ink, weight);
}
