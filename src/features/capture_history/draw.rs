//! 【截图历史】【窗口绘制】列表缩略图、透明预览和统一确认控件

use super::{
    layout::{Control, ROW, rect},
    window::{Focus, HistoryWindow},
};
use crate::{
    renderer::{Painter, text::TextStyle},
    theme::{color, font},
    ui::{controls, file_picker},
};
use tiny_skia::{FilterQuality, Pixmap, PixmapPaint, Rect, Transform};

/// 【截图历史】【透明图片】将 image 等比绘制到 canvas 的 bounds，scale 为设备比例；透明区域显示小棋盘
fn image(canvas: &mut Pixmap, image: &Pixmap, bounds: Rect, scale: f32) {
    let fit = (bounds.width() / image.width() as f32).min(bounds.height() / image.height() as f32);
    let (w, h) = (image.width() as f32 * fit, image.height() as f32 * fit);
    let (x, y) = (
        bounds.x() + (bounds.width() - w) * 0.5,
        bounds.y() + (bounds.height() - h) * 0.5,
    );
    {
        let mut painter = Painter::new(canvas).with_scale(scale);
        for row in 0..(h / 10.0).ceil() as usize {
            for column in 0..(w / 10.0).ceil() as usize {
                let (dx, dy) = (column as f32 * 10.0, row as f32 * 10.0);
                painter.fill_rounded(
                    rect(x + dx, y + dy, (w - dx).min(10.0), (h - dy).min(10.0)),
                    0.0,
                    if (row + column) % 2 == 0 {
                        color::border()
                    } else {
                        color::field()
                    },
                );
            }
        }
    }
    canvas.draw_pixmap(
        0,
        0,
        image.as_ref(),
        &PixmapPaint {
            quality: FilterQuality::Bilinear,
            ..Default::default()
        },
        Transform::from_scale(fit * scale, fit * scale).post_translate(x * scale, y * scale),
        None,
    );
}

/// 【截图历史】【操作可用】根据 state 的任务与选择判断 control 是否可执行；关闭与清空始终可用
fn enabled(state: &HistoryWindow, control: Control) -> bool {
    if control == Control::Close {
        return true;
    }
    if state.jobs.foreground() {
        return false;
    }
    match control {
        Control::Copy | Control::Edit | Control::Pin | Control::Save | Control::Delete => {
            state.model.selected().is_some()
        }
        Control::Minus => state.settings.limit > 1,
        Control::Plus => state.settings.limit < 200,
        _ => true,
    }
}

/// 【截图历史】【历史界面】按 state、scale 绘制 canvas；不解码图像，不进行磁盘读取
pub(super) fn draw(state: &mut HistoryWindow, canvas: &mut Pixmap, scale: f32) {
    canvas.fill(color::panel().color());
    let availability: Vec<_> = state
        .layout
        .buttons
        .iter()
        .map(|(control, _)| enabled(state, *control))
        .collect();
    {
        let mut painter = Painter::new(canvas)
            .with_scale(scale)
            .with_text(&mut state.fonts, &mut state.cache);
        let layout = &state.layout;
        painter.text(
            rect(12.0, 4.0, layout.size.0 - 80.0, 30.0),
            "Mark Shot / Screenshot History",
            TextStyle::new(font::label(), color::foreground()),
        );
        for (index, &(control, bounds)) in layout.buttons.iter().enumerate() {
            let label = if control == Control::Enabled {
                if state.settings.enabled {
                    "Recording: On"
                } else {
                    "Recording: Off"
                }
            } else {
                control.label()
            };
            controls::button(
                &mut painter,
                bounds,
                label,
                control == Control::Enabled && state.settings.enabled,
                state.hovered == Some(control),
                availability[index],
            );
        }
        painter.text(
            rect(124.0, 42.0, 30.0, 28.0),
            "Keep",
            TextStyle::new(font::small(), color::muted()),
        );
        painter.fill_rounded(layout.search, 4.0, color::field());
        painter.fill_rounded(layout.limit, 4.0, color::field());
        if state.search.text().is_empty() && state.focus != Focus::Search {
            painter.text(
                layout.search_text,
                "Search date, size, filename or indexed text",
                TextStyle::new(font::small(), color::muted()),
            );
        }
        let summary = format!(
            "{} shown / {} captures / {} MiB limit",
            state.model.visible.len(),
            state.model.entries.len(),
            state.settings.max_size_mib
        );
        painter.text(
            rect(10.0, 112.0, layout.size.0 - 20.0, 20.0),
            &summary,
            TextStyle::new(font::small(), color::muted()),
        );
        painter.fill_rounded(layout.list, 4.0, color::field());
        painter.fill_rounded(layout.preview, 4.0, color::field());
        for (row, index) in state
            .model
            .visible
            .iter()
            .skip(state.model.scroll)
            .take(layout.rows())
            .enumerate()
        {
            let entry = &state.model.entries[*index];
            let selected = state.model.selected.as_deref() == Some(&entry.name);
            let bounds = rect(
                layout.list.x() + 3.0,
                layout.list.y() + row as f32 * ROW + 2.0,
                layout.list.width() - 6.0,
                ROW - 4.0,
            );
            if selected {
                painter.fill_rounded(bounds, 4.0, color::active());
            }
            let fg = if selected {
                color::on_active()
            } else {
                color::foreground()
            };
            let width = bounds.width() - 90.0;
            let title = entry
                .created
                .with_timezone(&chrono::Local)
                .format("%m-%d %H:%M:%S")
                .to_string();
            painter.text(
                rect(bounds.x() + 86.0, bounds.y() + 2.0, width, 20.0),
                &title,
                TextStyle::new(font::small(), fg),
            );
            let size = format!(
                "{}x{} / {} KiB",
                entry.width,
                entry.height,
                entry.bytes.div_ceil(1024)
            );
            let label = controls::fit_label(&mut painter, &size, font::small(), width);
            painter.text(
                rect(bounds.x() + 86.0, bounds.y() + 22.0, width, 18.0),
                &label,
                TextStyle::new(font::small(), fg),
            );
            if !entry.search.is_empty() {
                let label = controls::fit_label(&mut painter, &entry.search, font::small(), width);
                painter.text(
                    rect(bounds.x() + 86.0, bounds.y() + 38.0, width, 18.0),
                    &label,
                    TextStyle::new(font::small(), fg),
                );
            }
        }
        if state.model.visible.is_empty() {
            painter.text(
                layout.list,
                if state.model.query.is_empty() {
                    "No screenshot history"
                } else {
                    "No matching screenshots"
                },
                TextStyle::new(font::small(), color::muted()).center(),
            );
        }
        if let Some(entry) = state.model.selected() {
            let label = format!(
                "{}x{} / scale {} / {} KiB",
                entry.width,
                entry.height,
                entry.scale,
                entry.bytes.div_ceil(1024)
            );
            let label = controls::fit_label(
                &mut painter,
                &label,
                font::small(),
                layout.preview.width() - 12.0,
            );
            painter.text(
                rect(
                    layout.preview.x() + 6.0,
                    layout.preview.y() + 3.0,
                    layout.preview.width() - 12.0,
                    20.0,
                ),
                &label,
                TextStyle::new(font::small(), color::muted()),
            );
        }
        let status = state
            .status
            .as_deref()
            .unwrap_or("Ctrl+F: search / Ctrl+C: copy / Enter: edit / Ctrl+S: save as");
        let label = controls::fit_label(&mut painter, status, font::small(), layout.size.0 - 34.0);
        painter.text(
            rect(10.0, layout.size.1 - 28.0, layout.size.0 - 34.0, 22.0),
            &label,
            TextStyle::new(font::small(), color::muted()),
        );
    }
    // 1. 【截图历史】【按需预览】只绘制已加载的小图；原始 PNG 留在磁盘用于复制和保存
    for (row, index) in state
        .model
        .visible
        .iter()
        .skip(state.model.scroll)
        .take(state.layout.rows())
        .enumerate()
    {
        let entry = &state.model.entries[*index];
        let bounds = rect(
            state.layout.list.x() + 8.0,
            state.layout.list.y() + row as f32 * ROW + 8.0,
            74.0,
            46.0,
        );
        if let Some(thumbnail) = state.model.thumbnail(&entry.name) {
            image(canvas, thumbnail, bounds, scale);
        } else {
            Painter::new(canvas)
                .with_scale(scale)
                .fill_rounded(bounds, 3.0, color::border());
        }
    }
    if let Some(entry) = state.model.selected() {
        let bounds = state.layout.preview;
        let inner = rect(
            bounds.x() + 8.0,
            bounds.y() + 27.0,
            bounds.width() - 16.0,
            (bounds.height() - 35.0).max(1.0),
        );
        if let Some(preview) = state.model.preview(&entry.name) {
            image(canvas, preview, inner, scale);
        } else {
            let mut painter = Painter::new(canvas)
                .with_scale(scale)
                .with_text(&mut state.fonts, &mut state.cache);
            painter.text(
                inner,
                if state.model.failed.contains(&entry.name) {
                    "Preview unavailable"
                } else {
                    "Loading preview..."
                },
                TextStyle::new(font::small(), color::muted()).center(),
            );
        }
    }
    state.search.draw(
        &mut state.fonts,
        &mut state.cache,
        canvas,
        state.layout.search_text,
        scale,
        state.focused
            && state.focus == Focus::Search
            && !state.clear_confirmation
            && state.picker.is_none(),
    );
    state.limit.draw(
        &mut state.fonts,
        &mut state.cache,
        canvas,
        state.layout.limit_text,
        scale,
        state.focused
            && state.focus == Focus::Limit
            && !state.clear_confirmation
            && state.picker.is_none(),
    );
    // 2. 【截图历史】【确认弹层】默认选择取消，只覆盖当前窗口，不使用外部原生交互
    if state.clear_confirmation {
        let mut painter = Painter::new(canvas)
            .with_scale(scale)
            .with_text(&mut state.fonts, &mut state.cache);
        let panel = state.layout.confirm;
        painter.fill_rounded(panel, 6.0, color::panel());
        painter.panel_border(
            panel.x(),
            panel.y(),
            panel.width(),
            panel.height(),
            6.0,
            1.0,
        );
        painter.text(
            rect(
                panel.x() + 12.0,
                panel.y() + 10.0,
                panel.width() - 24.0,
                26.0,
            ),
            "Clear screenshot history?",
            TextStyle::new(font::label(), color::foreground()),
        );
        painter.text(
            rect(
                panel.x() + 12.0,
                panel.y() + 44.0,
                panel.width() - 24.0,
                24.0,
            ),
            "Delete the local history records.",
            TextStyle::new(font::small(), color::muted()),
        );
        painter.text(
            rect(
                panel.x() + 12.0,
                panel.y() + 70.0,
                panel.width() - 24.0,
                24.0,
            ),
            "Saved files and pinned images will be kept.",
            TextStyle::new(font::small(), color::muted()),
        );
        for &(control, bounds) in &state.layout.confirm_buttons {
            controls::button(
                &mut painter,
                bounds,
                control.label(),
                (control == Control::ConfirmClear) == state.confirm_clear,
                state.hovered == Some(control),
                true,
            );
        }
    }
    if let Some(picker) = &mut state.picker {
        file_picker::draw::draw(picker, &mut state.fonts, &mut state.cache, canvas, scale);
    }
}
