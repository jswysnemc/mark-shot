//! 【截图历史】【窗口绘制】组合原版框架、透明预览、记录视口与受控确认层

use super::{
    chrome,
    layout::{Control, rect},
    window::HistoryWindow,
};
use crate::{
    renderer::{Painter, text::TextStyle},
    ui::{
        controls::{self, ButtonStyle},
        file_picker,
    },
};
use tiny_skia::{FilterQuality, Pixmap, PixmapPaint, Rect, Transform};

/// 【截图历史】【图片绘制】把 image 等比绘制到 canvas 的 bounds，scale 为物理比例，透明像素直接合成到面板
pub(super) fn image(canvas: &mut Pixmap, image: &Pixmap, bounds: Rect, scale: f32) {
    let fit = (bounds.width() / image.width() as f32).min(bounds.height() / image.height() as f32);
    let (width, height) = (image.width() as f32 * fit, image.height() as f32 * fit);
    let (x, y) = (
        bounds.x() + (bounds.width() - width) * 0.5,
        bounds.y() + (bounds.height() - height) * 0.5,
    );
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

/// 【截图历史】【清空确认】在 canvas 内绘制 state 的确认层，scale 为缓冲比例；保留默认取消和独占输入
fn confirmation(state: &mut HistoryWindow, canvas: &mut Pixmap, scale: f32) {
    if !state.clear_confirmation {
        return;
    }
    let mut painter = Painter::new(canvas)
        .with_scale(scale)
        .with_text(&mut state.fonts, &mut state.cache);
    let panel = state.layout.confirm;
    controls::frame(&mut painter, panel, chrome::BACKGROUND, chrome::BORDER, 6.0);
    painter.text(
        rect(
            panel.x() + 12.0,
            panel.y() + 10.0,
            panel.width() - 24.0,
            26.0,
        ),
        "Clear History",
        TextStyle::new(40.0 / 3.0, chrome::FOREGROUND),
    );
    if let Some(buffer) = chrome::status_buffer(
        &mut state.fonts,
        Some("Delete all screenshot history? Saved files will be kept."),
        panel.width() - 24.0,
    ) {
        let mut painter = Painter::new(canvas)
            .with_scale(scale)
            .with_text(&mut state.fonts, &mut state.cache);
        painter.text_buffer(
            &buffer,
            (panel.x() + 12.0, panel.y() + 44.0),
            chrome::FOREGROUND.color(),
        );
    }
    let mut painter = Painter::new(canvas)
        .with_scale(scale)
        .with_text(&mut state.fonts, &mut state.cache);
    for (control, bounds) in state.layout.confirm_buttons {
        controls::icon_button(
            &mut painter,
            bounds,
            control.label(),
            None,
            ButtonStyle {
                fill: chrome::HOVER,
                foreground: chrome::FOREGROUND,
                border: if (control == Control::ConfirmClear) == state.confirm_clear {
                    chrome::SELECTED
                } else {
                    chrome::BORDER
                },
                font_size: 40.0 / 3.0,
                weight: cosmic_text::Weight(500),
            },
        );
    }
}

/// 【截图历史】【历史绘制】按 state 与 scale 绘制 canvas；只读取已有缓存，不初始化 OCR 或执行磁盘读取
pub(super) fn draw(state: &mut HistoryWindow, canvas: &mut Pixmap, scale: f32) {
    canvas.fill(chrome::BACKGROUND.color());
    chrome::draw(state, canvas, scale);
    super::list_draw::draw(state, canvas, scale);
    let preview = state.layout.preview;
    controls::frame(
        &mut Painter::new(canvas).with_scale(scale),
        preview,
        chrome::BACKGROUND,
        chrome::BORDER,
        6.0,
    );
    // 1. 【截图历史】【预览内容】原版样式内边距七像素，再扣除窗口预览的六像素留白
    let inner = rect(
        preview.x() + 13.0,
        preview.y() + 13.0,
        (preview.width() - 26.0).max(1.0),
        (preview.height() - 26.0).max(1.0),
    );
    if let Some(entry) = state.model.selected()
        && let Some(preview) = state.model.preview(&entry.name)
    {
        image(canvas, preview, inner, scale);
    } else {
        let label = if state.model.entries.is_empty() {
            "No screenshots in history"
        } else if state
            .model
            .selected()
            .is_some_and(|entry| state.model.failed.contains(&entry.name))
        {
            "Screenshot is no longer available"
        } else {
            "Loading preview..."
        };
        Painter::new(canvas)
            .with_scale(scale)
            .with_text(&mut state.fonts, &mut state.cache)
            .text(
                inner,
                label,
                TextStyle::new(40.0 / 3.0, chrome::FOREGROUND).center(),
            );
    }
    // 2. 【截图历史】【交互隔离】确认层及文件选择器最后绘制，底层操作由输入路由阻止穿透
    confirmation(state, canvas, scale);
    if let Some(picker) = &mut state.picker {
        file_picker::draw::draw(picker, &mut state.fonts, &mut state.cache, canvas, scale);
    }
}
