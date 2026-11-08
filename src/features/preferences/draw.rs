//! 【设置窗口】【紧凑布局】分类导航、可搜索设置行和内嵌选项菜单共用原生控件

use super::{
    catalog::{ENTRIES, Kind, Page, value_label},
    window::{Control, Preferences},
};
use crate::{
    renderer::{Painter, text::TextStyle},
    ui::controls,
};
use tiny_skia::{Pixmap, Rect};

/// 【设置窗口】【矩形构造】按位置和尺寸返回非空控件矩形，布局只使用逻辑坐标
fn rect(left: f32, top: f32, width: f32, height: f32) -> Rect {
    Rect::from_xywh(left, top, width.max(1.0), height.max(1.0)).unwrap()
}
/// 【设置窗口】【导航宽度】按 state 窗口宽度返回紧凑侧栏宽度
pub(super) fn sidebar(state: &Preferences) -> f32 {
    if state.size.0 < 720 { 144.0 } else { 176.0 }
}
/// 【设置窗口】【分类位置】按 state 和 index 返回导航项矩形，关于入口固定于底部
pub(super) fn nav_rect(state: &Preferences, index: usize) -> Rect {
    rect(
        12.0,
        if index == Page::ALL.len() - 1 {
            state.size.1 as f32 - 58.0
        } else {
            92.0 + index as f32 * 44.0
        },
        sidebar(state) - 24.0,
        36.0,
    )
}
/// 【设置窗口】【搜索位置】按 state 返回搜索框范围
pub(super) fn search_rect(state: &Preferences) -> Rect {
    rect(
        sidebar(state) + 24.0,
        22.0,
        state.size.0 as f32 - sidebar(state) - 88.0,
        34.0,
    )
}

/// 【设置窗口】【关闭位置】按 state 返回右上角关闭按钮范围
pub(super) fn close_rect(state: &Preferences) -> Rect {
    rect(state.size.0 as f32 - 56.0, 22.0, 32.0, 34.0)
}
/// 【设置窗口】【属性位置】按 state 和 row 返回可编辑值矩形
pub(super) fn value_rect(state: &Preferences, row: usize) -> Rect {
    let width = ((state.size.0 as f32 - sidebar(state)) * 0.42).clamp(160.0, 260.0);
    rect(
        state.size.0 as f32 - width - 28.0,
        148.0 + row as f32 * 68.0,
        width,
        34.0,
    )
}
/// 【设置窗口】【确认位置】按 state 和 row 返回文本确认按钮矩形
pub(super) fn apply_rect(state: &Preferences, row: usize) -> Rect {
    let value = value_rect(state, row);
    rect(value.right() - 48.0, value.top(), 48.0, value.height())
}
/// 【设置窗口】【路径位置】按 state 返回高级页配置路径控件
pub(super) fn path_rect(state: &Preferences) -> Rect {
    rect(
        sidebar(state) + 24.0,
        state.size.1 as f32 - 92.0,
        state.size.0 as f32 - sidebar(state) - 48.0,
        30.0,
    )
}
/// 【设置窗口】【菜单位置】按 state、choice 和 count 返回菜单项矩形，末端越界时向上展开
pub(super) fn choice_rect(state: &Preferences, choice: usize, count: usize) -> Rect {
    let row = state
        .visible()
        .iter()
        .position(|index| Some(*index) == state.choices)
        .unwrap_or(0)
        .saturating_sub(state.scroll);
    let value = value_rect(state, row);
    let top = (value.bottom() + 4.0).min(state.size.1 as f32 - 46.0 - count as f32 * 34.0);
    rect(
        value.left(),
        top + choice as f32 * 34.0,
        value.width(),
        34.0,
    )
}

/// 【设置窗口】【完整绘制】把 state 按 scale 绘入 canvas，字体和色彩继承统一主题，文字字段最后绘制光标
pub(super) fn draw(state: &mut Preferences, canvas: &mut Pixmap, scale: f32) {
    let chinese = state.chinese();
    let toolbar_page = state.toolbar_page();
    let palette = crate::theme::window::Palette::current();
    let visible = state.visible();
    let display_values: Vec<_> = (0..ENTRIES.len()).map(|index| state.value(index)).collect();
    let capacity = state.capacity();
    let scrollbar = state.scrollbar();
    let labels = ENTRIES
        .iter()
        .map(|entry| format!("{} {}", entry.label(chinese), entry.help(chinese)))
        .collect::<Vec<_>>()
        .join(" ");
    crate::fonts::ui_fallback::ensure_text(
        &mut state.fonts,
        &format!(
            "{labels} 工具栏 增删与拖动排序；关闭设置后立即应用 设置 搜索设置 外观与语言 截图与标注 快捷键 文字识别 高级 关于 即时保存 默认值 配置路径已复制 点击选择 开启 关闭 按下快捷键 修改设置后自动保存，下次截图生效 已保存 {}",
            state.status
        ),
    );
    canvas.fill(palette.window.color());
    let side = sidebar(state);
    let search = search_rect(state);
    let close = close_rect(state);
    let mut fields = Vec::new();
    {
        let hover = state.hit(state.pointer);
        let mut painter = Painter::new(canvas)
            .with_scale(scale)
            .with_text(&mut state.fonts, &mut state.cache);
        painter.fill_rounded(
            rect(0.0, 0.0, side, state.size.1 as f32),
            0.0,
            palette.alternate,
        );
        if let Some(bar) = scrollbar {
            painter.fill_rounded(bar.track, 4.0, palette.alternate);
            painter.fill_rounded(
                bar.handle,
                3.0,
                if state.scrollbar_drag.is_some() || hover == Some(Control::Scrollbar) {
                    palette.accent
                } else {
                    palette.muted
                },
            );
        }
        painter.text(
            rect(20.0, 22.0, side - 40.0, 26.0),
            "Mark Shot",
            TextStyle::new(17.0, palette.text).weight(cosmic_text::Weight::SEMIBOLD),
        );
        painter.text(
            rect(20.0, 50.0, side - 40.0, 22.0),
            if chinese { "设置" } else { "Settings" },
            TextStyle::new(12.0, palette.muted),
        );
        for (index, page) in Page::ALL.into_iter().enumerate() {
            let area = rect(
                12.0,
                if index == Page::ALL.len() - 1 {
                    state.size.1 as f32 - 58.0
                } else {
                    92.0 + index as f32 * 44.0
                },
                side - 24.0,
                36.0,
            );
            let active = state.page == page && state.search.text().is_empty();
            controls::button(
                &mut painter,
                area,
                page.title(chinese),
                active,
                hover == Some(Control::Page(page)),
                true,
            );
        }
        controls::frame(
            &mut painter,
            search,
            palette.base,
            if state.search_focus {
                palette.accent
            } else {
                palette.border
            },
            6.0,
        );
        controls::frame(
            &mut painter,
            close,
            if hover == Some(Control::Close) {
                palette.alternate
            } else {
                palette.window
            },
            palette.window,
            5.0,
        );
        let mut cross = tiny_skia::PathBuilder::new();
        cross.move_to(close.left() + 11.0, close.top() + 12.0);
        cross.line_to(close.left() + 21.0, close.top() + 22.0);
        cross.move_to(close.left() + 21.0, close.top() + 12.0);
        cross.line_to(close.left() + 11.0, close.top() + 22.0);
        painter.stroke_path(
            &cross.finish().unwrap(),
            palette.muted.color(),
            &tiny_skia::Stroke {
                width: 1.5,
                line_cap: tiny_skia::LineCap::Round,
                ..Default::default()
            },
        );
        if state.search.text().is_empty() && !state.search_focus {
            painter.text(
                rect(
                    search.x() + 12.0,
                    search.y(),
                    search.width() - 24.0,
                    search.height(),
                ),
                if chinese {
                    "搜索设置…  Ctrl+F"
                } else {
                    "Search settings…  Ctrl+F"
                },
                TextStyle::new(13.0, palette.muted),
            );
        }
        painter.text(
            rect(side + 24.0, 78.0, state.size.0 as f32 - side - 48.0, 30.0),
            if state.search.text().is_empty() {
                state.page.title(chinese)
            } else if chinese {
                "搜索结果"
            } else {
                "Search results"
            },
            TextStyle::new(20.0, palette.text).weight(cosmic_text::Weight::SEMIBOLD),
        );
        painter.text(
            rect(side + 24.0, 111.0, state.size.0 as f32 - side - 48.0, 22.0),
            if toolbar_page && chinese {
                "增删与拖动排序；关闭设置后立即应用"
            } else if toolbar_page {
                "Add, remove and reorder; applies when returning to the editor."
            } else if chinese {
                "自动保存；快捷键、保存与通知即时生效"
            } else {
                "Auto-save; shortcuts, saving and notifications apply immediately."
            },
            TextStyle::new(12.0, palette.muted),
        );
        for (row, index) in visible
            .iter()
            .copied()
            .skip(state.scroll)
            .take(capacity)
            .enumerate()
        {
            let entry = &ENTRIES[index];
            let width = ((state.size.0 as f32 - side) * 0.42).clamp(160.0, 260.0);
            let value = rect(
                state.size.0 as f32 - width - 28.0,
                148.0 + row as f32 * 68.0,
                width,
                34.0,
            );
            let title = rect(side + 24.0, value.y(), value.left() - side - 42.0, 34.0);
            let label =
                controls::fit_label(&mut painter, entry.label(chinese), 14.0, title.width());
            painter.text(title, &label, TextStyle::new(14.0, palette.text));
            let help = controls::fit_label(
                &mut painter,
                entry.help(chinese),
                11.0,
                state.size.0 as f32 - side - 52.0,
            );
            painter.text(
                rect(
                    side + 24.0,
                    value.bottom() + 1.0,
                    state.size.0 as f32 - side - 52.0,
                    15.0,
                ),
                &help,
                TextStyle::new(11.0, palette.muted),
            );
            painter.fill_rounded(
                rect(
                    side + 24.0,
                    value.bottom() + 15.0,
                    state.size.0 as f32 - side - 48.0,
                    1.0,
                ),
                0.0,
                palette.border,
            );
            if state
                .editing
                .as_ref()
                .is_some_and(|(editing, _)| *editing == index)
            {
                controls::frame(&mut painter, value, palette.base, palette.accent, 5.0);
                if matches!(entry.kind, Kind::Shortcut) {
                    painter.text(
                        value,
                        if chinese {
                            "按下快捷键…"
                        } else {
                            "Press a shortcut…"
                        },
                        TextStyle::new(13.0, palette.accent).center(),
                    );
                } else {
                    fields.push((
                        index,
                        rect(
                            value.x() + 8.0,
                            value.y() + 3.0,
                            value.width() - 62.0,
                            value.height() - 6.0,
                        ),
                    ));
                    controls::button(
                        &mut painter,
                        rect(value.right() - 48.0, value.y(), 48.0, value.height()),
                        if chinese { "保存" } else { "Save" },
                        true,
                        false,
                        true,
                    );
                }
            } else {
                let value_text = &display_values[index];
                let caption = controls::fit_label(
                    &mut painter,
                    value_label(value_text, chinese),
                    14.0,
                    value.width() - 20.0,
                );
                controls::button(
                    &mut painter,
                    value,
                    &caption,
                    matches!(entry.kind, Kind::Toggle) && value_text == "true",
                    hover == Some(Control::Row(index)) || state.focus == Some(index),
                    true,
                );
                if matches!(entry.kind, Kind::Choice(_)) {
                    let mut path = tiny_skia::PathBuilder::new();
                    path.move_to(value.right() - 13.0, value.y() + 14.0);
                    path.line_to(value.right() - 9.0, value.y() + 18.0);
                    path.line_to(value.right() - 5.0, value.y() + 14.0);
                    painter.stroke_path(
                        &path.finish().unwrap(),
                        palette.muted.color(),
                        &tiny_skia::Stroke {
                            width: 1.3,
                            line_cap: tiny_skia::LineCap::Round,
                            ..Default::default()
                        },
                    );
                }
            }
        }
        if state.page == Page::About && state.search.text().is_empty() {
            painter.text(
                rect(side + 24.0, 158.0, 350.0, 32.0),
                concat!("Mark Shot ", env!("CARGO_PKG_VERSION")),
                TextStyle::new(17.0, palette.text),
            );
            painter.text(
                rect(side + 24.0, 199.0, 350.0, 26.0),
                if chinese {
                    "截图、标注、滚动截图与文字识别"
                } else {
                    "Capture, annotate, scroll and recognize text"
                },
                TextStyle::new(13.0, palette.muted),
            );
        }
        if state.page == Page::Advanced && state.search.text().is_empty() {
            let area = rect(
                side + 24.0,
                state.size.1 as f32 - 92.0,
                state.size.0 as f32 - side - 48.0,
                30.0,
            );
            let path = controls::fit_label(
                &mut painter,
                &state.path.to_string_lossy(),
                12.0,
                area.width() - 14.0,
            );
            controls::button(
                &mut painter,
                area,
                &path,
                false,
                hover == Some(Control::ConfigPath),
                true,
            );
        }
        let status = if toolbar_page && state.status.is_empty() {
            if chinese {
                "移除按钮不会禁用快捷键；分隔线可重复添加"
            } else {
                "Removing a button keeps its shortcut; separators may repeat"
            }
            .into()
        } else if state.status.is_empty() {
            format!(
                "{} / {}",
                (state.scroll + capacity).min(visible.len()),
                visible.len()
            )
        } else {
            state.status.clone()
        };
        let status = controls::fit_label(
            &mut painter,
            &status,
            12.0,
            state.size.0 as f32 - side - 48.0,
        );
        painter.text(
            rect(
                side + 24.0,
                state.size.1 as f32 - 40.0,
                state.size.0 as f32 - side - 48.0,
                24.0,
            ),
            &status,
            TextStyle::new(12.0, palette.muted),
        );
    }
    if toolbar_page {
        super::toolbar::draw::draw(state, canvas, scale);
    }
    if state.search_focus || !state.search.text().is_empty() {
        state.search.draw(
            &mut state.fonts,
            &mut state.cache,
            canvas,
            rect(
                search.x() + 10.0,
                search.y() + 3.0,
                search.width() - 20.0,
                search.height() - 6.0,
            ),
            scale,
            state.search_focus,
        );
    }
    for (index, area) in fields {
        if let Some((editing, field)) = &mut state.editing
            && *editing == index
        {
            field.draw(
                &mut state.fonts,
                &mut state.cache,
                canvas,
                area,
                scale,
                true,
            );
        }
    }
    if let Some(index) = state.choices
        && let Kind::Choice(choices) = ENTRIES[index].kind
    {
        let areas = (0..choices.len())
            .map(|choice| choice_rect(state, choice, choices.len()))
            .collect::<Vec<_>>();
        let hover = state.hit(state.pointer);
        let mut painter = Painter::new(canvas)
            .with_scale(scale)
            .with_text(&mut state.fonts, &mut state.cache);
        for (choice, value) in choices.iter().enumerate() {
            controls::frame(
                &mut painter,
                areas[choice],
                palette.base,
                palette.border,
                4.0,
            );
            controls::button(
                &mut painter,
                areas[choice],
                value_label(value, chinese),
                choice == state.choice_cursor,
                hover == Some(Control::Choice(choice)),
                true,
            );
        }
    }
}
