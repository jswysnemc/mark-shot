//! 【文字识别】【结果绘制】组合单行标题栏、透明正文、表格分区和轻量页脚

use tiny_skia::{PathBuilder, Pixmap, Rect, Stroke};

use super::{
    layout::Control,
    window::{ResultWindow, View},
};
use crate::{
    renderer::{Painter, text::TextStyle},
    theme::Rgba,
    ui::controls,
};

/// 【文字识别】【窗口绘制】按 state 活动页绘制 canvas，scale 为文字及图像的缓冲比例，无返回值
pub(super) fn draw(state: &mut ResultWindow, canvas: &mut Pixmap, scale: f32) {
    state.refresh_layout();
    let buttons = super::titlebar_draw::DrawState::new(state);
    let colors = super::palette::Palette::current();
    canvas.fill(tiny_skia::Color::TRANSPARENT);
    // 1. 【文字识别】【窗口框架】正文透明，标题和窗口边框沿用原版紧凑尺寸
    {
        let mut painter = Painter::new(canvas)
            .with_scale(scale)
            .with_text(&mut state.fonts, &mut state.cache)
            .with_icons(&state.icons);
        controls::frame(
            &mut painter,
            Rect::from_xywh(0.0, 0.0, state.layout.size.0, state.layout.size.1).unwrap(),
            colors.window,
            colors.border,
            10.0,
        );
        super::titlebar_draw::draw(
            &buttons,
            &state.layout,
            state.feedback.label(super::feedback::Pane::Source),
            &mut painter,
        );
        if state.view == View::Text
            && state.focused
            && state.source_menu.is_none()
            && state.translation.language_menu.is_none()
            && state.focus_target == super::control_input::FocusTarget::Content
        {
            controls::frame(
                &mut painter,
                state.layout.editor_frame,
                Rgba(0, 0, 0, 0),
                colors.border,
                4.0,
            );
        }
        if state.view == View::Table {
            painter.clipped(Some(state.layout.table_header.title), |painter| {
                painter.text(
                    state.layout.table_header.title,
                    state.language.text("Table"),
                    TextStyle::new(13.0, colors.text).weight(cosmic_text::Weight(600)),
                );
            });
            if let Some(table) = &state.table {
                painter.clipped(Some(state.layout.table_header.summary), |painter| {
                    painter.text(
                        state.layout.table_header.summary,
                        &state.language.format(
                            "%1 rows × %2 columns",
                            &[
                                &table.table.rows.len().to_string(),
                                &table.table.columns().to_string(),
                            ],
                        ),
                        TextStyle::new(12.0, colors.muted),
                    );
                });
            }
            super::titlebar_draw::button(
                &mut painter,
                &buttons,
                Control::TableCopy,
                state.layout.table_copy,
                state.feedback.label(super::feedback::Pane::Table),
                None,
                13.0,
            );
        }
    }
    // 2. 【文字识别】【内容页面】切页只改变绘制入口，不销毁正文及撤销历史
    let focused = state.focused
        && state.source_menu.is_none()
        && state.translation.language_menu.is_none()
        && state.focus_target == super::control_input::FocusTarget::Content;
    match state.view {
        View::Text => state.source.draw(
            &mut state.fonts,
            &mut state.cache,
            canvas,
            state.layout.text_rect(),
            scale,
            focused,
        ),
        View::Table => {
            if let Some(table) = &mut state.table {
                table.draw(
                    &mut state.fonts,
                    &mut state.cache,
                    canvas,
                    state.layout.table_body,
                    scale,
                    focused,
                );
            }
        }
        View::Image => super::source_preview_draw::draw(
            &mut state.preview,
            &state.image,
            canvas,
            state.layout.body,
            scale,
        ),
    }
    if state.view == View::Text {
        super::translation::draw::draw(state, canvas, scale);
    }
    // 3. 【文字识别】【页脚入口】没有状态时尺寸入口位于页脚中部，不增加默认快捷键文字
    let mut painter = Painter::new(canvas)
        .with_scale(scale)
        .with_text(&mut state.fonts, &mut state.cache);
    if let Some(status) = &state.status {
        let label = controls::fit_label(
            &mut painter,
            state.language.text(status),
            12.0,
            state.layout.size.0 - 46.0,
        );
        painter.text(
            Rect::from_xywh(
                10.0,
                state.layout.size.1 - 22.0,
                state.layout.size.0 - 46.0,
                18.0,
            )
            .unwrap(),
            &label,
            TextStyle::new(12.0, colors.muted),
        );
    }
    let grip = state.layout.grip;
    let mut path = PathBuilder::new();
    for offset in [5.0, 9.0, 13.0] {
        path.move_to(grip.right() - offset, grip.bottom() - 4.0);
        path.line_to(grip.right() - 4.0, grip.bottom() - offset);
    }
    if let Some(path) = path.finish() {
        painter.stroke_path(
            &path,
            colors.muted,
            &Stroke {
                width: 1.0,
                ..Stroke::default()
            },
        );
    }
}
