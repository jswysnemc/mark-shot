//! 【工具栏设置】【页面绘制】复用实际 SVG 图标，紧凑预览支持换行，全部输入区域与排版共用

use super::super::{shortcuts, window::Preferences};
use super::{
    input::shortcut_key,
    layout::{Hit, Layout, rect},
};
use crate::{
    renderer::{Painter, text::TextStyle},
    theme::window::Palette,
    ui::{
        controls,
        toolbar::{ToolbarItem, catalog, icons},
    },
};
use tiny_skia::{Pixmap, Rect};

/// 【工具栏设置】【按钮文案】按 hit、chinese 和 vertical 返回操作文字，图标项目由目录提供
fn label(hit: Hit, chinese: bool, vertical: bool) -> &'static str {
    match hit {
        Hit::Profile(profile) => profile.label(chinese),
        Hit::Reset => {
            if chinese {
                "恢复默认按钮"
            } else {
                "Reset buttons"
            }
        }
        Hit::Left => {
            if chinese {
                "向前移动"
            } else {
                "Move earlier"
            }
        }
        Hit::Right => {
            if chinese {
                "向后移动"
            } else {
                "Move later"
            }
        }
        Hit::Remove => {
            if chinese {
                "移除按钮"
            } else {
                "Remove"
            }
        }
        Hit::Shortcut => {
            if chinese {
                "录入快捷键"
            } else {
                "Record shortcut"
            }
        }
        Hit::ClearShortcut => {
            if chinese {
                "清除快捷键"
            } else {
                "Clear shortcut"
            }
        }
        Hit::ResetShortcut => {
            if chinese {
                "默认快捷键"
            } else {
                "Default shortcut"
            }
        }
        Hit::Direction => match (chinese, vertical) {
            (true, true) => "纵向排列",
            (true, false) => "横向排列",
            (false, true) => "Vertical",
            (false, false) => "Horizontal",
        },
        _ => "",
    }
}

/// 【工具栏设置】【项目图标】把 item 绘入 painter 的 area，palette 决定图标与分隔线颜色
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

/// 【工具栏设置】【内容绘制】按 scale 绘制 state 页面到 canvas，仅裁切内容区域，滚动条保持常驻
pub(in crate::features::preferences) fn draw(
    state: &mut Preferences,
    canvas: &mut Pixmap,
    scale: f32,
) {
    state.toolbar.prepare();
    let layout = Layout::new(state);
    let chinese = state.chinese();
    let palette = Palette::current();
    let hover = layout.hit(state.pointer);
    let profile = state.toolbar.profile;
    let items = profile.items(&state.toolbar.config);
    let selected = state.toolbar.selected.filter(|index| *index < items.len());
    let key = shortcut_key(state);
    let focused = selected.or({
        if let Some(Hit::Item(index)) = hover {
            Some(index)
        } else {
            None
        }
    });
    let name = focused
        .and_then(|index| catalog::entry(items[index]))
        .map_or(if chinese { "分隔线" } else { "Separator" }, |entry| {
            entry.label(chinese)
        });
    let caption = if state.toolbar.recording {
        if chinese {
            "按下新的快捷键，Esc 取消".to_owned()
        } else {
            "Press a shortcut; Esc to cancel".to_owned()
        }
    } else if focused.is_some() {
        let shortcut = focused
            .and_then(|index| catalog::entry(items[index]))
            .and_then(|entry| entry.shortcut())
            .map(|key| shortcuts::text(&state.values, key))
            .filter(|text| !text.is_empty());
        format!(
            "{} · {}",
            name,
            shortcut.as_deref().unwrap_or(if chinese {
                "未设置快捷键"
            } else {
                "No shortcut"
            })
        )
    } else if chinese {
        "单击选中；拖动调整顺序，也可使用 Alt+方向键".into()
    } else {
        "Select a button; drag or use Alt+arrows to reorder".into()
    };
    let labels = catalog::ENTRIES
        .iter()
        .map(|entry| entry.label(chinese))
        .collect::<Vec<_>>()
        .join(" ");
    crate::fonts::ui_fallback::ensure_text(
        &mut state.fonts,
        &format!(
            "{labels} {caption} 当前工具栏 添加按钮 恢复默认按钮 向前移动 向后移动 移除按钮 横向排列 纵向排列 录入快捷键 清除快捷键 默认快捷键 截图主栏 截图操作栏 图片 全屏 分隔线"
        ),
    );
    let mut painter = Painter::new(canvas)
        .with_scale(scale)
        .with_icons(&state.toolbar.icons)
        .with_text(&mut state.fonts, &mut state.cache);
    painter.clipped(Some(layout.viewport), |painter| {
        let title_top = layout.controls[0].1.top() + 44.0;
        painter.text(
            rect(layout.preview.left(), title_top, 200.0, 26.0),
            if chinese {
                "当前工具栏"
            } else {
                "Current toolbar"
            },
            TextStyle::new(13.0, palette.text).weight(cosmic_text::Weight::SEMIBOLD),
        );
        controls::frame(painter, layout.preview, palette.base, palette.border, 8.0);
        for (index, item) in items.iter().enumerate() {
            let area = layout.cells[index];
            let active = selected == Some(index);
            if active || hover == Some(Hit::Item(index)) {
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
        if let Some(drag) = &state.toolbar.drag
            && drag.active
        {
            let target = drag.target.min(layout.cells.len());
            let cell = if target < layout.cells.len() {
                layout.cells[target]
            } else {
                *layout.cells.last().unwrap()
            };
            let horizontal = if target == layout.cells.len() {
                cell.right()
            } else {
                cell.left() - 2.0
            };
            painter.fill_rounded(
                rect(horizontal, cell.top(), 2.0, cell.height()),
                1.0,
                palette.accent,
            );
        }
        let caption = controls::fit_label(painter, &caption, 12.0, layout.selected.width());
        painter.text(
            layout.selected,
            &caption,
            TextStyle::new(
                12.0,
                if state.toolbar.recording {
                    palette.accent
                } else {
                    palette.muted
                },
            ),
        );
        for (hit, area) in &layout.controls {
            let enabled = match hit {
                Hit::Left => selected.is_some_and(|index| index > 0),
                Hit::Right => selected.is_some_and(|index| index + 1 < items.len()),
                Hit::Remove => {
                    selected.is_some()
                        && items
                            .iter()
                            .filter(|item| matches!(item, ToolbarItem::Button(_)))
                            .count()
                            > usize::from(selected.is_some_and(|index| {
                                matches!(items[index], ToolbarItem::Button(_))
                            }))
                }
                Hit::Shortcut | Hit::ClearShortcut | Hit::ResetShortcut => key.is_some(),
                _ => true,
            };
            controls::button(
                painter,
                *area,
                label(*hit, chinese, profile.vertical(&state.toolbar.config)),
                *hit == Hit::Profile(profile) || (*hit == Hit::Shortcut && state.toolbar.recording),
                hover == Some(*hit),
                enabled,
            );
        }
        painter.text(
            layout.add_title,
            if chinese {
                "添加按钮"
            } else {
                "Add buttons"
            },
            TextStyle::new(13.0, palette.text).weight(cosmic_text::Weight::SEMIBOLD),
        );
        for (item, area) in &layout.available {
            controls::frame(
                painter,
                *area,
                if hover == Some(Hit::Add(*item)) {
                    palette.pressed
                } else {
                    palette.alternate
                },
                palette.window,
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
    });
    if let Some(bar) = layout.scrollbar {
        painter.fill_rounded(bar.track, 4.0, palette.alternate);
        painter.fill_rounded(bar.handle, 3.0, palette.muted);
    }
}
