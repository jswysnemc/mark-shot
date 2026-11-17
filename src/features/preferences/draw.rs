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
    let about_viewport = super::about::viewport(state);
    let toolbar_position = super::toolbar_preview::position(&state.values);
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
                entry.note(&state.source, &state.values, chinese)
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
    state.fonts.ensure_text(&format!("{labels} {group_labels} {} {} 保存 恢复 实际导出预览 已添加外框 原图 外框关闭 按下快捷键 浅色 深色 Windows 系统 OCR 管理语言包与运行库 配置工具栏布局 配置此服务 打开 未找到匹配设置 清空搜索或尝试其他关键词 截图 标注 文字识别 翻译 原生窗口，无需浏览器运行环境 版本 操作系统 桌面环境 窗口管理器 合成器 显示协议 未识别", values.join(" "), state.resource_note));
    if layout.toolbar_preview.is_some() {
        state.fonts.ensure_text(
            "位置预览 跟随选区 屏幕顶部 屏幕底部 屏幕左侧 屏幕右侧 选区 主栏 侧栏 属性栏",
        );
    }
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
            if let Some(bounds) = layout.toolbar_preview {
                super::toolbar_preview::draw(painter, bounds, toolbar_position, chinese);
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
                let note = if entry.section == "desktop" && entry.key != "autostart" {
                    let message = if entry.key == "tray_enabled" {
                        &state.desktop_status.tray
                    } else {
                        &state.desktop_status.shortcuts
                    };
                    message.label(chinese)
                } else if entry.section == "ocr" && entry.key == "resources" {
                    state.resource_note.clone()
                } else {
                    entry.note(&state.source, &state.values, chinese)
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
                    let shortcut = matches!(entry.kind, Kind::Shortcut);
                    let edit = layout::text_edit(row.control);
                    widgets::field(
                        painter,
                        if shortcut { row.control } else { edit.field },
                        "",
                        true,
                        false,
                    );
                    widgets::button(
                        painter,
                        if shortcut {
                            layout::reset_rect(row.control)
                        } else {
                            edit.reset
                        },
                        if chinese { "恢复" } else { "Reset" },
                        false,
                        hover == Some(Control::Reset(row.index)),
                        true,
                    );
                    if shortcut {
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
                        widgets::button(
                            painter,
                            edit.save,
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
                } else if matches!(entry.kind, Kind::Link(_)) {
                    let link_label = match (entry.section, entry.key) {
                        ("ocr", "resources") => {
                            if chinese {
                                "管理语言包与运行库"
                            } else {
                                "Manage models & runtime"
                            }
                        }
                        ("toolbar_layout", "layout") => {
                            if chinese {
                                "配置工具栏布局"
                            } else {
                                "Configure toolbar"
                            }
                        }
                        ("translation", _) => {
                            if chinese {
                                "配置此服务"
                            } else {
                                "Configure service"
                            }
                        }
                        _ => {
                            if chinese {
                                "打开"
                            } else {
                                "Open"
                            }
                        }
                    };
                    widgets::action_link(painter, row.control, link_label, focused);
                } else {
                    let label = entry.option_label(&values[row.index], chinese);
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
                super::about::draw(
                    painter,
                    about_viewport,
                    chinese,
                    &state.about,
                    hover == Some(Control::Repository),
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
