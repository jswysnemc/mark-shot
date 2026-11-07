//! 【截图历史】【窗口绘制】组合原版框架、透明预览、记录视口与文件选择器

use super::{chrome, layout::rect, window::HistoryWindow};
use crate::{
    renderer::{Painter, text::TextStyle},
    ui::{controls, file_picker},
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

/// 【截图历史】【历史绘制】按 state 与 scale 绘制 canvas；只读取已有缓存，不初始化 OCR 或执行磁盘读取
pub(super) fn draw(state: &mut HistoryWindow, canvas: &mut Pixmap, scale: f32) {
    canvas.fill(chrome::background().color());
    chrome::draw(state, canvas, scale);
    super::list_draw::draw(state, canvas, scale);
    let preview = state.layout.preview;
    controls::frame(
        &mut Painter::new(canvas).with_scale(scale),
        preview,
        chrome::background(),
        chrome::border(),
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
                TextStyle::new(40.0 / 3.0, chrome::foreground()).center(),
            );
    }
    // 2. 【截图历史】【交互隔离】文件选择器最后绘制，清空确认由独立模态窗口处理
    if let Some(picker) = &mut state.picker {
        file_picker::draw::draw(picker, &mut state.fonts, &mut state.cache, canvas, scale);
    }
}
