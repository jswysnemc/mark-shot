//! 【OCR 翻译】【分区绘制】语言输入、译文历史和进度均局限在译文区域，不修改原文绘制状态

use super::super::{
    control_input::FocusTarget,
    layout::{Control, contains},
    window::ResultWindow,
};
use crate::{
    renderer::{Painter, text::TextStyle},
    theme::Rgba,
    ui::controls,
};
use tiny_skia::Rect;

#[cfg(test)]
mod tests;

/// 【OCR 翻译】【分区内容】按 window、canvas 和 scale 绘制译文与操作行，无返回值，忙碌编辑器仍可显示选择但不显示输入光标
pub(crate) fn draw(window: &mut ResultWindow, canvas: &mut tiny_skia::Pixmap, scale: f32) {
    let Some(layout) = &window.layout.translation else {
        return;
    };
    let colors = super::super::palette::Palette::current();
    let focused = window.focused
        && window.source_menu.is_none()
        && window.translation.language_menu.is_none();
    let busy = window.translation.state.busy();
    let buttons = super::super::titlebar_draw::DrawState::new(window);
    let enabled = busy || window.source.has_non_whitespace();
    let down = window.control_down(Control::Translate);
    {
        let mut painter = Painter::new(canvas)
            .with_scale(scale)
            .with_text(&mut window.fonts, &mut window.cache)
            .with_icons(&window.icons);
        if let Some(caption) = layout.caption {
            painter.text(
                caption,
                window.language.text("Target Language"),
                TextStyle::new(12.0, colors.muted),
            );
        }
        let language_focus = focused && window.focus_target == FocusTarget::Language;
        let hovered = window
            .pointer
            .is_some_and(|position| contains(layout.language, position));
        controls::frame(
            &mut painter,
            layout.language,
            if busy { colors.alternate } else { colors.base },
            if !busy && (language_focus || hovered) {
                colors.accent
            } else {
                colors.border
            },
            6.0,
        );
        let arrow = layout.arrow;
        let mut path = tiny_skia::PathBuilder::new();
        path.move_to(arrow.x() + 8.0, arrow.y() + 10.0);
        path.line_to(arrow.x() + 12.0, arrow.y() + 14.0);
        path.line_to(arrow.x() + 16.0, arrow.y() + 10.0);
        if let Some(path) = path.finish() {
            painter.stroke_path(
                &path,
                if busy { colors.muted } else { colors.text },
                &tiny_skia::Stroke {
                    width: 1.5,
                    line_cap: tiny_skia::LineCap::Round,
                    line_join: tiny_skia::LineJoin::Round,
                    ..Default::default()
                },
            );
        }
        let hovered = window.hovered == Some(Control::Translate);
        controls::icon_button(
            &mut painter,
            layout.translate,
            window
                .language
                .text(if busy { "Cancel" } else { "Translate" }),
            None,
            controls::ButtonStyle {
                fill: if !enabled || down {
                    colors.alternate
                } else {
                    colors.accent
                },
                border: if !enabled {
                    colors.border
                } else if hovered {
                    colors.text
                } else {
                    colors.accent
                },
                foreground: if !enabled {
                    colors.muted
                } else if down {
                    colors.text
                } else {
                    colors.on_accent
                },
                font_size: 13.0,
                weight: cosmic_text::Weight(600),
            },
        );
        let title_width = (layout.copy.left()
            - layout.header.left()
            - if layout.undo.is_some() { 36.0 } else { 4.0 })
        .max(1.0);
        painter.clipped(
            Some(
                Rect::from_xywh(
                    layout.header.x() + 2.0,
                    layout.header.y(),
                    title_width,
                    21.0,
                )
                .unwrap(),
            ),
            |painter| {
                painter.text(
                    Rect::from_xywh(
                        layout.header.x() + 2.0,
                        layout.header.y(),
                        title_width,
                        21.0,
                    )
                    .unwrap(),
                    window.language.text("Translated text"),
                    TextStyle::new(12.0, colors.text).weight(cosmic_text::Weight(600)),
                );
            },
        );
        super::super::titlebar_draw::button(
            &mut painter,
            &buttons,
            Control::TranslationCopy,
            layout.copy,
            window
                .feedback
                .label(super::super::feedback::Pane::Translation),
            Some(crate::ui::icons::COPY),
            12.0,
        );
        if let Some(undo) = layout.undo {
            super::super::titlebar_draw::button(
                &mut painter,
                &buttons,
                Control::TranslationUndo,
                undo,
                "",
                Some(super::super::titlebar_draw::UNDO),
                12.0,
            );
        }
        if focused && window.focus_target == FocusTarget::Translation {
            controls::frame(
                &mut painter,
                layout.frame,
                Rgba(0, 0, 0, 0),
                colors.border,
                4.0,
            );
        }
        if let Some(progress) = layout.progress {
            painter.fill_rounded(progress, 1.0, colors.alternate);
            let length = progress.width() * 0.4;
            let x = progress.x() + (progress.width() + length) * window.translation.animation_phase
                - length;
            painter.clipped(Some(progress), |painter| {
                painter.fill_rounded(
                    Rect::from_xywh(x, progress.y(), length, 3.0).unwrap(),
                    1.0,
                    colors.accent,
                )
            });
        }
        if let Some(notice) = layout.notice
            && let Some(buffer) = &layout.notice_text
        {
            let color = if window.translation.state.error() {
                colors.error
            } else {
                colors.muted
            };
            painter.clipped(Some(notice), |painter| {
                painter.text_buffer(buffer, (notice.x(), notice.y()), color.color())
            });
        }
        if window.translation.characters == 0 {
            painter.text(
                Rect::from_xywh(
                    layout.editor.x(),
                    layout.editor.y(),
                    layout.editor.width(),
                    20.0,
                )
                .unwrap(),
                window.language.text("Translation appears here"),
                TextStyle::new(15.0, colors.muted),
            );
        }
    }
    let translation_rect = layout.editor;
    let language_rect = layout.language_text;
    window.translation.editor.draw(
        &mut window.fonts,
        &mut window.cache,
        canvas,
        translation_rect,
        scale,
        focused && window.focus_target == FocusTarget::Translation && !busy,
    );
    window.translation.language.draw_single_line(
        &mut window.fonts,
        &mut window.cache,
        canvas,
        language_rect,
        scale,
        focused && window.focus_target == FocusTarget::Language && !busy,
        &mut window.translation.language_offset,
    );
    let mut painter = Painter::new(canvas)
        .with_scale(scale)
        .with_text(&mut window.fonts, &mut window.cache);
    let statistics = format!(
        "{}: {}",
        window.language.text("Translated text"),
        window.language.format(
            "%1 characters",
            &[&window.translation.characters.to_string()]
        )
    );
    let width = painter.measure(&statistics, 12.0).round();
    painter.text(
        Rect::from_xywh(
            window.layout.grip.x() - 4.0 - width,
            window.layout.size.1 - 21.0,
            width,
            17.0,
        )
        .unwrap(),
        &statistics,
        TextStyle::new(12.0, colors.muted),
    );
}
