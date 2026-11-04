//! 【文字识别】【结果绘制】结果窗口的紧凑工具栏、原图和页面内容

use tiny_skia::{FilterQuality, PathBuilder, Pixmap, PixmapPaint, Rect, Stroke, Transform};

use super::{
    layout::Control,
    window::{ResultWindow, View},
};
use crate::{
    renderer::{Painter, text::TextStyle},
    theme::{Rgba, color, font, radius},
    ui::controls,
};

/// 【文字识别】【按钮状态绘制】用 painter、rect 和 label 绘制 down、hovered、enabled 状态；按下沿用原版暗色背景和正文前景，无返回值
fn button(
    painter: &mut Painter<'_>,
    rect: Rect,
    label: &str,
    down: bool,
    hovered: bool,
    enabled: bool,
) {
    if down && enabled {
        controls::icon_button(
            painter,
            rect,
            label,
            None,
            controls::ButtonStyle {
                fill: Rgba(22, 61, 64, 255),
                foreground: color::foreground(),
                border: Rgba(0, 0, 0, 0),
                font_size: font::label(),
                weight: cosmic_text::Weight::NORMAL,
            },
        );
    } else {
        controls::button(painter, rect, label, false, hovered, enabled);
    }
}

/// 【文字识别】【原图预览】把 image 等比绘制到 canvas 的 body；scale 是物理比例，透明像素显示棋盘底板
fn image(canvas: &mut Pixmap, image: &Pixmap, body: Rect, scale: f32) {
    let fit = ((body.width() - 20.0) / image.width() as f32)
        .min((body.height() - 20.0) / image.height() as f32);
    let (width, height) = (image.width() as f32 * fit, image.height() as f32 * fit);
    let (x, y) = (
        body.x() + (body.width() - width) / 2.0,
        body.y() + (body.height() - height) / 2.0,
    );
    {
        let mut painter = Painter::new(canvas).with_scale(scale);
        for row in 0..(height / 12.0).ceil() as usize {
            for column in 0..(width / 12.0).ceil() as usize {
                let (dx, dy) = (column as f32 * 12.0, row as f32 * 12.0);
                let rect = Rect::from_xywh(
                    x + dx,
                    y + dy,
                    (width - dx).min(12.0),
                    (height - dy).min(12.0),
                )
                .unwrap();
                let fill = if (row + column) % 2 == 0 {
                    color::border()
                } else {
                    color::field()
                };
                painter.fill_rounded(rect, 0.0, fill);
            }
        }
    }
    canvas.draw_pixmap(
        0,
        0,
        image.as_ref(),
        &PixmapPaint {
            quality: FilterQuality::Bilinear,
            ..PixmapPaint::default()
        },
        Transform::from_scale(fit * scale, fit * scale).post_translate(x * scale, y * scale),
        None,
    );
}

/// 【文字识别】【窗口绘制】按 state 的活动页面绘制 canvas；scale 用于文字与原图的分数缩放
pub(super) fn draw(state: &mut ResultWindow, canvas: &mut Pixmap, scale: f32) {
    canvas.fill(color::panel().color());
    let can_undo = state.can_undo();
    let copy_enabled = state.control_enabled(Control::Copy);
    let close_down = state.control_down(Control::Close);
    let copy_down = state.control_down(Control::Copy);
    let undo_down = state.control_down(Control::Undo);
    // 1. 【文字识别】【窗口框架】绘制共用标题、页签、操作按钮和正文底板
    {
        let mut painter = Painter::new(canvas)
            .with_scale(scale)
            .with_text(&mut state.fonts, &mut state.cache);
        let layout = &state.layout;
        painter.text(
            Rect::from_xywh(12.0, 4.0, layout.size.0 - 84.0, 30.0).unwrap(),
            "Mark Shot / OCR",
            TextStyle::new(font::label(), color::foreground()),
        );
        button(
            &mut painter,
            layout.close,
            "Close",
            close_down,
            state.hovered == Some(Control::Close),
            true,
        );
        for &(view, rect) in &layout.tabs {
            controls::button(
                &mut painter,
                rect,
                view.label(),
                view == state.view,
                state.hovered == Some(Control::View(view)),
                true,
            );
        }
        button(
            &mut painter,
            layout.copy,
            if state.copied { "Copied" } else { "Copy" },
            copy_down,
            state.hovered == Some(Control::Copy),
            copy_enabled,
        );
        if let Some(rect) = layout.undo.filter(|_| can_undo) {
            button(
                &mut painter,
                rect,
                "Undo",
                undo_down,
                state.hovered == Some(Control::Undo),
                can_undo,
            );
        }
        if state.focused
            && let super::control_input::FocusTarget::Button(control) = state.focus_target
        {
            let rect = match control {
                Control::Close => Some(layout.close),
                Control::Copy if copy_enabled => Some(layout.copy),
                Control::Undo if can_undo => layout.undo,
                _ => None,
            };
            if let Some(rect) = rect {
                crate::ui::text_properties::draw::border(
                    &mut painter,
                    rect,
                    6.0,
                    1.0,
                    Rgba(94, 234, 212, 255),
                );
            }
        }
        painter.fill_rounded(layout.body, radius::item().min(6.0), color::field());
    }

    // 2. 【文字识别】【内容页面】保留编辑器实例，仅切换当前绘制内容
    match state.view {
        View::Text => state.source.draw(
            &mut state.fonts,
            &mut state.cache,
            canvas,
            state.layout.editor,
            scale,
            state.focused
                && !state.menu
                && state.focus_target == super::control_input::FocusTarget::Content,
        ),
        View::Table => {
            if let Some(table) = &mut state.table {
                table.draw(
                    &mut state.fonts,
                    &mut state.cache,
                    canvas,
                    state.layout.body,
                    scale,
                    state.focused
                        && !state.menu
                        && state.focus_target == super::control_input::FocusTarget::Content,
                );
            }
        }
        View::Image => image(canvas, &state.image, state.layout.body, scale),
    }

    // 3. 【文字识别】【反馈菜单】复制反馈与格式菜单共用按钮样式，不调用原生浏览器交互
    let mut painter = Painter::new(canvas)
        .with_scale(scale)
        .with_text(&mut state.fonts, &mut state.cache);
    let hint = match state.view {
        View::Text => "Ctrl+Shift+C: copy all text",
        View::Table => "Ctrl+Shift+C: copy table as TSV",
        View::Image => "Copy: original image",
    };
    let status = state.status.as_deref().unwrap_or(hint);
    let label = controls::fit_label(
        &mut painter,
        status,
        font::small(),
        state.layout.size.0 - 40.0,
    );
    painter.text(
        Rect::from_xywh(
            10.0,
            state.layout.size.1 - 24.0,
            state.layout.size.0 - 40.0,
            20.0,
        )
        .unwrap(),
        &label,
        TextStyle::new(font::small(), color::muted()),
    );
    if state.menu {
        painter.fill_rounded(
            Rect::from_xywh(state.layout.size.0 - 194.0, 74.0, 190.0, 128.0).unwrap(),
            6.0,
            color::panel(),
        );
        for &(format, rect) in &state.layout.formats {
            controls::button(
                &mut painter,
                rect,
                format.label(),
                false,
                state.hovered == Some(Control::Format(format)),
                true,
            );
        }
    }
    let (width, height) = state.layout.size;
    let mut grip = PathBuilder::new();
    for offset in [5.0, 9.0, 13.0] {
        grip.move_to(width - offset, height - 4.0);
        grip.line_to(width - 4.0, height - offset);
    }
    if let Some(path) = grip.finish() {
        painter.stroke_path(
            &path,
            color::muted(),
            &Stroke {
                width: 1.0,
                ..Stroke::default()
            },
        );
    }
}
