//! 【设置下拉】【浮层绘制】独立面板使用整行高亮与选中标记，浮层最后绘制以覆盖页面内容

use super::{
    super::{layout::rect, window::Preferences},
    layout::{Hit, Layout},
};
use crate::{
    renderer::{Painter, text::TextStyle},
    theme::window::Palette,
    ui::{controls, icons},
};
use tiny_skia::Pixmap;

/// 【设置下拉】【绘制入口】按 state 与 scale 把浮层绘入 canvas，当前值与键盘候选分别显示
pub(in crate::features::preferences) fn draw(
    state: &mut Preferences,
    canvas: &mut Pixmap,
    scale: f32,
) {
    let chinese = state.chinese();
    let Some(menu) = &mut state.dropdown else {
        return;
    };
    let layout = Layout::new(menu, state.size);
    let palette = Palette::current();
    let labels = menu
        .options
        .iter()
        .map(|option| format!("{} {}", option.label, option.detail))
        .collect::<Vec<_>>()
        .join(" ");
    state
        .fonts
        .ensure_text(&format!("{labels} 搜索选项 无匹配选项"));
    let mut painter = Painter::new(canvas)
        .with_scale(scale)
        .with_icons(&state.icons)
        .with_text(&mut state.fonts, &mut state.cache);
    painter.fill_rounded(
        rect(
            layout.bounds.left() - 2.0,
            layout.bounds.top() + 3.0,
            layout.bounds.width() + 4.0,
            layout.bounds.height() + 2.0,
        ),
        9.0,
        palette.alternate,
    );
    controls::frame(
        &mut painter,
        layout.bounds,
        palette.base,
        palette.border,
        8.0,
    );
    if let Some(search) = layout.search {
        controls::frame(&mut painter, search, palette.window, palette.accent, 5.0);
        painter.icon(
            icons::MAGNIFIER,
            13.0,
            search.left() + 9.0,
            search.top() + 8.0,
            palette.muted,
        );
        if menu
            .search
            .as_ref()
            .is_some_and(|field| field.text().is_empty())
        {
            painter.text(
                rect(
                    search.left() + 29.0,
                    search.top(),
                    search.width() - 37.0,
                    search.height(),
                ),
                if chinese {
                    "搜索选项…"
                } else {
                    "Search options…"
                },
                TextStyle::new(11.0, palette.muted),
            );
        }
    }
    for (index, bounds) in &layout.rows {
        let option = &menu.options[*index];
        let selected = menu.selected == Some(*index);
        let hovered = layout.hit(state.pointer) == Some(Hit::Option(*index));
        let focused = menu.cursor == Some(*index);
        if hovered || focused || selected {
            painter.fill_rounded(
                *bounds,
                5.0,
                if selected || focused {
                    palette.pressed
                } else {
                    palette.window
                },
            );
        }
        if selected {
            painter.icon(
                icons::CHECK,
                14.0,
                bounds.left() + 9.0,
                bounds.top() + 10.0,
                palette.accent,
            );
        }
        let has_detail = !option.detail.is_empty();
        let title = controls::fit_label(&mut painter, &option.label, 12.0, bounds.width() - 42.0);
        painter.text(
            rect(
                bounds.left() + 30.0,
                bounds.top() + if has_detail { 1.0 } else { 0.0 },
                bounds.width() - 38.0,
                if has_detail { 18.0 } else { 34.0 },
            ),
            &title,
            TextStyle::new(
                12.0,
                if selected {
                    palette.accent
                } else {
                    palette.text
                },
            ),
        );
        if has_detail {
            let detail =
                controls::fit_label(&mut painter, &option.detail, 10.0, bounds.width() - 42.0);
            painter.text(
                rect(
                    bounds.left() + 30.0,
                    bounds.top() + 18.0,
                    bounds.width() - 38.0,
                    14.0,
                ),
                &detail,
                TextStyle::new(10.0, palette.muted),
            );
        }
    }
    if layout.rows.is_empty() {
        painter.text(
            layout.list,
            if chinese {
                "无匹配选项"
            } else {
                "No matching options"
            },
            TextStyle::new(12.0, palette.muted).center(),
        );
    }
    if let Some(bar) = layout.scrollbar {
        painter.fill_rounded(bar.handle, 3.0, palette.muted);
    }
    if let Some(bounds) = layout.search
        && let Some(search) = &mut menu.search
    {
        search.draw(
            &mut state.fonts,
            &mut state.cache,
            canvas,
            rect(
                bounds.left() + 29.0,
                bounds.top() + 3.0,
                bounds.width() - 37.0,
                24.0,
            ),
            scale,
            true,
        );
    }
}
