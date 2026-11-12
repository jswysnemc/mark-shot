//! 【扫码窗口】【绘制】使用统一按钮、文本选择和窄滚动条，不显示大幅预览

use super::{
    ResultWindow,
    layout::{Control, Layout},
};
use crate::{
    renderer::{Painter, text::TextStyle},
    theme::window::Palette,
    ui::controls,
};

/// 【扫码窗口】【结果绘制】按 state、canvas 和 scale 绘制自适应结果窗口，返回空值
pub(super) fn draw(state: &mut ResultWindow, canvas: &mut tiny_skia::Pixmap, scale: f32) {
    let layout = Layout::new(state.size);
    let palette = Palette::current();
    let title = if state.chinese {
        "二维码与条形码"
    } else {
        "QR codes and barcodes"
    };
    let labels = if state.chinese {
        ["复制所选", "复制全部", "关闭"]
    } else {
        ["Copy selected", "Copy all", "Close"]
    };
    state
        .fonts
        .ensure_text(&format!("{title} {} {}", state.status, labels.join(" ")));
    state
        .editor
        .resize(&mut state.fonts, layout.text.width(), layout.text.height());
    let bar = state.editor.scrollbar(&mut state.fonts, layout.viewport);
    canvas.fill(palette.window.color());
    {
        let mut painter = Painter::new(canvas)
            .with_scale(scale)
            .with_text(&mut state.fonts, &mut state.cache);
        painter.text(
            layout.title,
            title,
            TextStyle::new(13.0, palette.text).weight(cosmic_text::Weight::SEMIBOLD),
        );
        painter.text(
            layout.status,
            &state.status,
            TextStyle::new(11.0, palette.muted),
        );
        for ((control, area), label) in layout.buttons.iter().zip(labels) {
            let enabled = match control {
                Control::Copy => state.editor.selected_text().is_some(),
                Control::CopyAll => !state.editor.is_empty(),
                Control::Close => true,
            };
            controls::button(
                &mut painter,
                *area,
                label,
                *control == Control::CopyAll,
                false,
                enabled,
            );
        }
        if let Some(bar) = bar {
            controls::button(&mut painter, bar.handle, "", false, false, true);
        }
    }
    state.editor.draw(
        &mut state.fonts,
        &mut state.cache,
        canvas,
        layout.text,
        scale,
        false,
    );
    crate::ui::window_frame::draw(canvas, scale);
}
