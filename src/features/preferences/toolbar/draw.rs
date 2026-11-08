//! 【工具栏设置】【内容绘制】同时展示主栏与侧栏，仅为选中项目显示移除操作

use super::super::{shortcuts, widgets, window::Preferences};
use super::layout::{Bar, Hit, Layout, rect};
use crate::{
    renderer::{Painter, text::TextStyle},
    theme::window::Palette,
    ui::{
        controls,
        toolbar::{ToolbarItem, catalog, icons},
    },
};
use tiny_skia::{Pixmap, Rect};

/// 【工具栏设置】【项目图标】把 item 绘入 painter 的 area，palette 决定图标与分隔线颜色，无返回值
fn icon(painter: &mut Painter<'_>, item: ToolbarItem, area: Rect, palette: Palette) {
    match item {
        ToolbarItem::Button(button) => painter.icon(
            icons::icon(button),
            18.0,
            area.left() + (area.width() - 18.0) / 2.0,
            area.top() + (area.height() - 18.0) / 2.0,
            palette.text,
        ),
        ToolbarItem::Seperator => painter.fill_rounded(
            rect(
                area.left() + area.width() / 2.0,
                area.top() + 8.0,
                1.0,
                area.height() - 16.0,
            ),
            0.5,
            palette.muted,
        ),
    }
}

/// 【工具栏设置】【工具说明】按 state 和 bar 的选择或悬停项目返回名称及快捷键，否则返回操作提示
fn caption(state: &Preferences, bar: &Bar, hover: Option<Hit>) -> String {
    let chinese = state.chinese();
    let selected = if state.toolbar.profile == bar.profile && state.toolbar.selected.is_some() {
        state.toolbar.selected
    } else if let Some(Hit::Item(profile, index)) = hover
        && profile == bar.profile
    {
        Some(index)
    } else if state.toolbar.profile == bar.profile {
        state.toolbar.selected
    } else {
        None
    };
    let items = bar.profile.items(&state.toolbar.config);
    if let Some(item) = selected.and_then(|index| items.get(index)) {
        let entry = catalog::entry(*item);
        let name = entry.map_or(if chinese { "分隔线" } else { "Separator" }, |entry| {
            entry.label(chinese)
        });
        let shortcut = entry
            .and_then(|entry| entry.shortcut())
            .map(|key| shortcuts::text(&state.values, key))
            .filter(|text| !text.is_empty());
        return shortcut.map_or_else(
            || name.to_owned(),
            |shortcut| format!("{name} · {shortcut}"),
        );
    }
    match (chinese, state.toolbar.profile == bar.profile) {
        (true, true) => "拖动排序 · Alt+方向键移动 · Delete 移除",
        (true, false) => "点击此栏，选择要添加的工具",
        (false, true) => "Drag to reorder · Alt+arrows · Delete to remove",
        (false, false) => "Select this toolbar to add tools",
    }
    .into()
}

/// 【工具栏设置】【预览绘制】按 bar、items、selected 和 hover 绘制 painter 中的按钮，palette 提供颜色，无返回值
fn preview(
    painter: &mut Painter<'_>,
    bar: &Bar,
    items: &[ToolbarItem],
    selected: Option<usize>,
    hover: Option<Hit>,
    palette: Palette,
    editor: &super::Editor,
    positions: &[(crate::config::toolbar_layout::Profile, usize, Rect)],
) {
    controls::frame(painter, bar.preview, palette.base, palette.border, 8.0);
    for (index, item) in items.iter().enumerate() {
        if editor.drag.as_ref().is_some_and(|drag| {
            drag.active && drag.source == super::drag::Source::Bar(bar.profile, index)
        }) {
            continue;
        }
        let target = positions
            .iter()
            .find(|entry| entry.0 == bar.profile && entry.1 == index)
            .map_or(bar.cells[index], |entry| entry.2);
        let area = editor.animation.position(bar.profile, index, target);
        let active = selected == Some(index);
        if active || hover == Some(Hit::Item(bar.profile, index)) {
            controls::frame(
                painter,
                area,
                if active {
                    palette.pressed
                } else {
                    palette.alternate
                },
                if active {
                    palette.accent
                } else {
                    palette.border
                },
                5.0,
            );
        }
        icon(painter, *item, area, palette);
    }
}

/// 【工具栏设置】【内容绘制】按 scale 绘制 state 页面到 canvas，仅裁切内容区域，滚动条保持常驻，无返回值
pub(in crate::features::preferences) fn draw(
    state: &mut Preferences,
    canvas: &mut Pixmap,
    scale: f32,
) {
    state.toolbar.prepare();
    let layout = Layout::new(state);
    let chinese = state.chinese();
    let palette = Palette::current();
    let hover = state.toolbar.keyboard.or_else(|| layout.hit(state.pointer));
    let positions = super::drag::positions(state, &layout);
    let shortcut_label = super::shortcuts::label(state);
    let profile = state.toolbar.profile;
    let captions: Vec<_> = layout
        .bars
        .iter()
        .map(|bar| caption(state, bar, hover))
        .collect();
    let pool_title = if chinese {
        format!("工具池 · 拖入任意栏，点击添加到{}", profile.label(true))
    } else {
        format!("Add to {}", profile.label(false))
    };
    let labels = catalog::ENTRIES
        .iter()
        .map(|entry| entry.label(chinese))
        .collect::<Vec<_>>()
        .join(" ");
    crate::fonts::ui_fallback::ensure_text(
        &mut state.fonts,
        &format!(
            "{labels} {} {pool_title} {shortcut_label} 主工具栏 侧工具栏 恢复默认 移除 分隔线",
            captions.join(" ")
        ),
    );
    let mut painter = Painter::new(canvas)
        .with_scale(scale)
        .with_icons(&state.toolbar.icons)
        .with_text(&mut state.fonts, &mut state.cache);
    painter.clipped(Some(layout.viewport), |painter| {
        for (bar, caption) in layout.bars.iter().zip(&captions) {
            let active = profile == bar.profile;
            controls::frame(
                painter,
                bar.panel,
                palette.base,
                if active {
                    palette.accent
                } else {
                    palette.border
                },
                8.0,
            );
            let items = bar.profile.items(&state.toolbar.config);
            preview(
                painter,
                bar,
                &items,
                if active { state.toolbar.selected } else { None },
                hover,
                palette,
                &state.toolbar,
                &positions,
            );
            if let Some(drag) = &state.toolbar.drag
                && drag.active
                && let Some(super::drag::Target::Bar(target_profile, index)) = drag.target
                && target_profile == bar.profile
            {
                controls::frame(
                    painter,
                    bar.slot(index),
                    palette.pressed,
                    palette.accent,
                    5.0,
                );
            }
            let caption = controls::fit_label(painter, caption, 12.0, bar.caption.width());
            painter.text(bar.caption, &caption, TextStyle::new(12.0, palette.muted));
        }
        for (hit, area) in &layout.controls {
            if let Hit::Profile(profile) = hit {
                painter.text(
                    *area,
                    profile.label(chinese),
                    TextStyle::new(12.0, palette.text).weight(cosmic_text::Weight::SEMIBOLD),
                );
                continue;
            }
            let label = match hit {
                Hit::Orientation(profile) => {
                    match (chinese, profile.vertical(&state.toolbar.config)) {
                        (true, true) => "纵向排列",
                        (true, false) => "横向排列",
                        (false, true) => "Vertical",
                        (false, false) => "Horizontal",
                    }
                }
                Hit::Profile(profile) => profile.label(chinese),
                Hit::Reset(_) => {
                    if chinese {
                        "恢复默认"
                    } else {
                        "Reset"
                    }
                }
                Hit::Remove => {
                    if chinese {
                        "移除"
                    } else {
                        "Remove"
                    }
                }
                Hit::Shortcut => &shortcut_label,
                _ => "",
            };
            let items = profile.items(&state.toolbar.config);
            let enabled = *hit != Hit::Remove
                || state
                    .toolbar
                    .selected
                    .and_then(|index| items.get(index))
                    .is_some_and(|selected| {
                        *selected == ToolbarItem::Seperator
                            || items
                                .iter()
                                .filter(|item| matches!(item, ToolbarItem::Button(_)))
                                .count()
                                > 1
                    });
            widgets::button(
                painter,
                *area,
                label,
                *hit == Hit::Shortcut && state.toolbar.recording.is_some(),
                hover == Some(*hit),
                enabled,
            );
        }
        widgets::panel(painter, layout.pool, &pool_title);
        for (item, area) in &layout.available {
            controls::frame(
                painter,
                *area,
                if hover == Some(Hit::Add(*item)) {
                    palette.pressed
                } else {
                    palette.base
                },
                palette.border,
                6.0,
            );
            icon(
                painter,
                *item,
                rect(area.left() + 6.0, area.top(), 28.0, area.height()),
                palette,
            );
            let title = catalog::entry(*item)
                .map_or(if chinese { "分隔线" } else { "Separator" }, |entry| {
                    entry.label(chinese)
                });
            let title = controls::fit_label(painter, title, 12.0, area.width() - 42.0);
            painter.text(
                rect(
                    area.left() + 38.0,
                    area.top(),
                    area.width() - 42.0,
                    area.height(),
                ),
                &title,
                TextStyle::new(12.0, palette.text),
            );
        }
        if let Some(drag) = &state.toolbar.drag
            && drag.active
        {
            let area = rect(
                state.pointer.0 as f32 - 16.0,
                state.pointer.1 as f32 - 16.0,
                32.0,
                32.0,
            );
            controls::frame(painter, area, palette.base, palette.accent, 5.0);
            icon(painter, drag.item(&state.toolbar.config), area, palette);
        }
    });
    if let Some(bar) = layout.scrollbar {
        painter.fill_rounded(bar.track, 4.0, palette.alternate);
        painter.fill_rounded(bar.handle, 3.0, palette.muted);
    }
}
