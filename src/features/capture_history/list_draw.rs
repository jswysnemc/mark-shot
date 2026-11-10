//! 【截图历史】【记录绘制】仅显示原版日期与尺寸两行，独立裁切列表和缩略图

use super::{chrome, draw::image, layout::rect, window::HistoryWindow};
use crate::{
    renderer::{Painter, text::TextStyle},
    ui::controls,
};
use tiny_skia::{Pixmap, PixmapPaint, Transform};

/// 【截图历史】【列表绘制】将 state 的可见行绘制到 canvas，scale 为设备比例，临时视口保证末行不覆盖底部按钮
pub(super) fn draw(state: &mut HistoryWindow, canvas: &mut Pixmap, scale: f32) {
    let viewport = state.layout.list;
    let Some(mut list) = Pixmap::new(
        (viewport.width() * scale).ceil() as u32,
        (viewport.height() * scale).ceil() as u32,
    ) else {
        return;
    };
    let scrollbar = state.scrollbar();
    let width = viewport.width() - if scrollbar.is_some() { 8.0 } else { 0.0 };
    let rows = (viewport.height() / super::layout::ROW).ceil() as usize;
    {
        let mut painter = Painter::new(&mut list)
            .with_scale(scale)
            .with_text(&mut state.fonts, &mut state.cache);
        for (row, index) in state
            .model
            .visible
            .iter()
            .skip(state.model.scroll)
            .take(rows)
            .enumerate()
        {
            let entry = &state.model.entries[*index];
            let bounds = chrome::row_rect(row, width);
            if state.model.selected.as_deref() == Some(&entry.name) {
                painter.fill_rounded(bounds, 6.0, chrome::selected());
            } else if state.hovered_row == Some(state.model.scroll + row) {
                painter.fill_rounded(bounds, 6.0, chrome::hover());
            }
            let title = entry
                .created
                .with_timezone(&chrono::Local)
                .format("%m-%d %H:%M:%S")
                .to_string();
            let size = format!("{} × {}", entry.width, entry.height);
            let text_width = (bounds.width() - 61.0).max(1.0);
            for (line, text) in [title, size].into_iter().enumerate() {
                let text = controls::fit_label(&mut painter, &text, 11.0, text_width);
                painter.text(
                    rect(
                        bounds.x() + 57.0,
                        bounds.y() + 2.0 + line as f32 * 16.0,
                        text_width,
                        16.0,
                    ),
                    &text,
                    TextStyle::new(
                        11.0,
                        if line == 0 {
                            chrome::foreground()
                        } else {
                            chrome::muted()
                        },
                    ),
                );
            }
        }
        if let Some(bar) = scrollbar {
            painter.fill_rounded(
                rect(
                    bar.handle.x() - viewport.x(),
                    bar.handle.y() - viewport.y(),
                    bar.handle.width(),
                    bar.handle.height(),
                ),
                3.0,
                chrome::border(),
            );
        }
    }
    for (row, index) in state
        .model
        .visible
        .iter()
        .skip(state.model.scroll)
        .take(rows)
        .enumerate()
    {
        let entry = &state.model.entries[*index];
        if let Some(thumbnail) = state.model.thumbnail(&entry.name) {
            image(
                &mut list,
                thumbnail,
                rect(7.0, 5.0 + row as f32 * super::layout::ROW, 44.0, 30.0),
                scale,
            );
        }
    }
    canvas.draw_pixmap(
        (viewport.x() * scale).round() as i32,
        (viewport.y() * scale).round() as i32,
        list.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        None,
    );
}
