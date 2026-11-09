//! 【设置窗口】【内容绘制】分组设置、真实控件与菜单共享布局，裁切滚动内容

use super::{
    catalog::{ENTRIES, Kind, Page, value_label},
    layout::{self, Layout, rect},
    widgets,
    window::{Control, Preferences},
};
use crate::{
    renderer::{Painter, text::TextStyle},
    theme::window::Palette,
    ui::controls,
};
use tiny_skia::Pixmap;

/// 【设置窗口】【完整绘制】按 state 和 scale 绘制 canvas，独立业务页面共享窗口外壳
pub(super) fn draw(state: &mut Preferences, canvas: &mut Pixmap, scale: f32) {
    super::shell::draw(state, canvas, scale);
    if state.recovery.is_some() {
        super::recovery::draw(state, canvas, scale);
        return;
    }
    if state.toolbar_page() {
        super::toolbar::draw::draw(state, canvas, scale);
        return;
    }
    if state.translation_page() {
        super::translation::draw::draw(state, canvas, scale);
        return;
    }
    let chinese = state.chinese();
    let palette = Palette::current();
    let layout = Layout::new(state);
    if layout.preview.is_some() {
        state
            .frame_preview
            .update(super::frame_preview::options(state));
    }
    let scrollbar = state.scrollbar();
    let hover = state.hit(state.pointer);
    let expanded = state.dropdown.as_ref().map(|menu| menu.target);
    let values: Vec<_> = (0..ENTRIES.len()).map(|index| state.value(index)).collect();
    let labels = ENTRIES
        .iter()
        .map(|entry| {
            format!(
                "{} {}",
                entry.label(chinese),
                entry.note(&state.source, chinese)
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    let group_labels = layout
        .groups
        .iter()
        .map(|group| group.title)
        .collect::<Vec<_>>()
        .join(" ");
    state.fonts.ensure_text(&format!("{labels} {group_labels} {} {} 保存 恢复 实际导出预览 已添加外框 原图 外框关闭 按下快捷键 浅色 深色 Windows 系统 OCR 未找到匹配设置 清空搜索或尝试其他关键词 截图 标注 文字识别 翻译 原生窗口，无需浏览器运行环境", values.join(" "), state.resource_note));
    let mut editors = Vec::new();
    {
        let mut painter = Painter::new(canvas)
            .with_scale(scale)
            .with_text(&mut state.fonts, &mut state.cache);
        painter.clipped(Some(layout.viewport), |painter| {
            for group in &layout.groups {
                widgets::panel(painter, group.bounds, group.title);
            }
            if let Some(bounds) = layout.preview {
                state.frame_preview.draw(painter, bounds, chinese);
            }
            for row in &layout.rows {
                if row.bounds.bottom() <= layout.viewport.top()
                    || row.bounds.top() >= layout.viewport.bottom()
                {
                    continue;
                }
                let entry = &ENTRIES[row.index];
                let numeric_editing = entry.step().is_some()
                    && state
                        .editing
                        .as_ref()
                        .is_some_and(|(index, _)| *index == row.index);
                let text_width = if row.stacked {
                    row.bounds.width() - 32.0
                } else {
                    row.control.left()
                        - row.bounds.left()
                        - 32.0
                        - if numeric_editing { 46.0 } else { 0.0 }
                };
                let label = controls::fit_label(painter, entry.label(chinese), 12.0, text_width);
                let note = if entry.section == "desktop" {
                    let message = if entry.key == "tray_enabled" {
                        &state.desktop_status.tray
                    } else {
                        &state.desktop_status.shortcuts
                    };
                    message.label(chinese)
                } else if entry.section == "ocr" && entry.key == "resources" {
                    state.resource_note.clone()
                } else {
                    entry.note(&state.source, chinese)
                };
                let help = controls::fit_label(
                    painter,
                    if state.search.text().is_empty() {
                        &note
                    } else {
                        entry.page.title(chinese)
                    },
                    10.0,
                    text_width,
                );
                painter.text(
                    rect(
                        row.bounds.left() + 16.0,
                        row.bounds.top() + if row.compact { 11.0 } else { 10.0 },
                        text_width,
                        22.0,
                    ),
                    &label,
                    TextStyle::new(12.0, palette.text),
                );
                if !row.compact && !matches!(entry.kind, Kind::Shortcut) {
                    painter.text(
                        rect(
                            row.bounds.left() + 16.0,
                            row.bounds.top() + 32.0,
                            text_width,
                            18.0,
                        ),
                        &help,
                        TextStyle::new(10.0, palette.muted),
                    );
                }
                if layout
                    .rows
                    .iter()
                    .any(|next| next.bounds.top() == row.bounds.bottom())
                {
                    painter.fill_rounded(
                        rect(
                            row.bounds.left() + 16.0,
                            row.bounds.bottom() - 1.0,
                            row.bounds.width() - 32.0,
                            1.0,
                        ),
                        0.0,
                        palette.window,
                    );
                }
                let editing = state
                    .editing
                    .as_ref()
                    .is_some_and(|(index, _)| *index == row.index);
                let focused =
                    hover == Some(Control::Row(row.index)) || state.focus == Some(row.index);
                if entry.step().is_some() {
                    let current = values[row.index].parse::<f64>().unwrap_or_default();
                    let (minimum, maximum) = entry.range().unwrap_or((0.0, i32::MAX as f64));
                    let direction = match hover {
                        Some(Control::Step(index, direction)) if index == row.index => direction,
                        _ => 0,
                    };
                    widgets::number(
                        painter,
                        row.control,
                        if editing { "" } else { &values[row.index] },
                        editing || focused,
                        (current > minimum, current < maximum),
                        direction,
                    );
                    if editing {
                        widgets::button(
                            painter,
                            layout::number_reset(row.control),
                            if chinese { "恢复" } else { "Reset" },
                            false,
                            hover == Some(Control::Reset(row.index)),
                            true,
                        );
                        editors.push((row.index, layout::editor_rect(row)));
                    }
                } else if matches!(entry.kind, Kind::Color) {
                    super::colors::draw(painter, row.control, &state.values, row.index, focused);
                } else if editing {
                    widgets::field(painter, row.control, "", true, false);
                    widgets::button(
                        painter,
                        layout::reset_rect(row.control),
                        if chinese { "恢复" } else { "Reset" },
                        false,
                        hover == Some(Control::Reset(row.index)),
                        true,
                    );
                    if matches!(entry.kind, Kind::Shortcut) {
                        widgets::button(
                            painter,
                            layout::apply_rect(row.control),
                            if entry.section == "desktop" {
                                if chinese { "禁用" } else { "Disable" }
                            } else if chinese {
                                "清除"
                            } else {
                                "Clear"
                            },
                            false,
                            hover == Some(Control::Clear(row.index)),
                            true,
                        );
                        painter.text(
                            rect(
                                row.control.left(),
                                row.control.top(),
                                (row.control.width() - 96.0).max(1.0),
                                row.control.height(),
                            ),
                            if chinese {
                                "按下快捷键…"
                            } else {
                                "Press a shortcut…"
                            },
                            TextStyle::new(12.0, palette.accent).center(),
                        );
                    } else {
                        let apply = layout::apply_rect(row.control);
                        widgets::button(
                            painter,
                            apply,
                            if chinese { "保存" } else { "Save" },
                            true,
                            hover == Some(Control::Apply),
                            true,
                        );
                        editors.push((row.index, layout::editor_rect(row)));
                    }
                } else if let Kind::Segments(choices) = entry.kind {
                    painter.fill_rounded(row.control, 6.0, palette.window);
                    for (choice, value) in choices.iter().copied().enumerate() {
                        widgets::button(
                            painter,
                            rect(
                                row.control.left()
                                    + choice as f32 * row.control.width() / choices.len() as f32
                                    + 3.0,
                                row.control.top() + 3.0,
                                row.control.width() / choices.len() as f32 - 6.0,
                                row.control.height() - 6.0,
                            ),
                            value_label(value, chinese),
                            values[row.index] == value,
                            hover == Some(Control::Segment(row.index, choice)),
                            true,
                        );
                    }
                } else if matches!(entry.kind, Kind::Toggle) {
                    widgets::switch(painter, row.control, values[row.index] == "true", focused);
                } else {
                    let label = if matches!(entry.kind, Kind::Link(_)) {
                        if chinese { "打开" } else { "Open" }
                    } else {
                        entry.option_label(&values[row.index], chinese)
                    };
                    if matches!(entry.kind, Kind::Choice(_)) {
                        widgets::select(
                            painter,
                            row.control,
                            label,
                            focused,
                            expanded == Some(super::dropdown::Target::Setting(row.index)),
                        );
                    } else {
                        widgets::field(painter, row.control, label, focused, false);
                    }
                }
            }
            if state.page == Page::About && state.search.text().is_empty() {
                let area = rect(
                    layout.viewport.left(),
                    layout.viewport.top(),
                    layout.viewport.width() - 14.0,
                    170.0,
                );
                widgets::panel(painter, area, "Mark Shot");
                painter.text(
                    rect(
                        area.left() + 16.0,
                        area.top() + 49.0,
                        area.width() - 32.0,
                        30.0,
                    ),
                    concat!("Mark Shot ", env!("CARGO_PKG_VERSION")),
                    TextStyle::new(16.0, palette.text).weight(cosmic_text::Weight::SEMIBOLD),
                );
                painter.text(
                    rect(
                        area.left() + 16.0,
                        area.top() + 87.0,
                        area.width() - 32.0,
                        24.0,
                    ),
                    if chinese {
                        "截图 · 标注 · 文字识别 · 翻译"
                    } else {
                        "Capture · Annotate · OCR · Translate"
                    },
                    TextStyle::new(12.0, palette.text),
                );
                painter.text(
                    rect(
                        area.left() + 16.0,
                        area.top() + 118.0,
                        area.width() - 32.0,
                        24.0,
                    ),
                    "GitHub: https://github.com/jswysnemc/mark-shot",
                    TextStyle::new(11.0, palette.muted),
                );
            } else if layout.rows.is_empty() {
                painter.text(
                    rect(
                        layout.viewport.left(),
                        layout.viewport.top() + 30.0,
                        layout.viewport.width(),
                        28.0,
                    ),
                    if chinese {
                        "未找到匹配设置"
                    } else {
                        "No matching settings"
                    },
                    TextStyle::new(14.0, palette.text).center(),
                );
                painter.text(
                    rect(
                        layout.viewport.left(),
                        layout.viewport.top() + 65.0,
                        layout.viewport.width(),
                        24.0,
                    ),
                    if chinese {
                        "清空搜索或尝试其他关键词"
                    } else {
                        "Clear the search or try another keyword"
                    },
                    TextStyle::new(11.0, palette.muted).center(),
                );
            }
        });
        if let Some(bar) = scrollbar {
            painter.fill_rounded(bar.track, 4.0, palette.alternate);
            painter.fill_rounded(bar.handle, 3.0, palette.muted);
        }
    }
    for (index, area) in editors {
        if area.top() >= layout.viewport.top()
            && area.bottom() <= layout.viewport.bottom()
            && let Some((active, editor)) = &mut state.editing
            && *active == index
        {
            editor.draw(
                &mut state.fonts,
                &mut state.cache,
                canvas,
                area,
                scale,
                true,
            );
        }
    }
}
