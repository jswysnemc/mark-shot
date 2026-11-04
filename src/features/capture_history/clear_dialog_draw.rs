//! 【截图历史】【确认呈现】原版清空对话框的紧凑布局、换行说明和统一按钮

use super::{
    chrome,
    clear_dialog::{Button, ClearDialog, MESSAGE},
    layout::{contains, rect},
};
use crate::{
    renderer::Painter,
    ui::controls::{self, ButtonStyle},
};
use cosmic_text::{Attrs, Buffer, Metrics, Shaping, Weight};
use tiny_skia::{Pixmap, Rect};

pub(super) struct Layout {
    pub label: Rect,
    pub buttons: [(Button, Rect); 2],
}

impl Layout {
    /// 【截图历史】【确认布局】按 size 保留原版十一像素边距、六像素间隔与三十像素按钮，返回逻辑矩形
    pub fn new(size: (u32, u32)) -> Self {
        let (width, height) = (size.0.max(223) as f32, size.1.max(77) as f32);
        let available = width - 28.0;
        let first = (available / 2.0).ceil();
        Self {
            label: rect(11.0, 11.0, width - 22.0, height - 58.0),
            buttons: [
                (Button::Cancel, rect(11.0, height - 41.0, first, 30.0)),
                (
                    Button::Clear,
                    rect(17.0 + first, height - 41.0, available - first, 30.0),
                ),
            ],
        }
    }

    /// 【截图历史】【确认命中】返回 position 所在按钮，说明和窗口空白不作为确认区域
    pub fn button(&self, position: (f64, f64)) -> Option<Button> {
        self.buttons
            .iter()
            .find(|(_, bounds)| contains(*bounds, position))
            .map(|(button, _)| *button)
    }
}

/// 【截图历史】【确认画面】将 state 的原版正文和按钮绘制到 canvas，scale 为设备比例，不加入额外标题或图标
pub(super) fn draw(state: &mut ClearDialog, canvas: &mut Pixmap, scale: f32) {
    canvas.fill(chrome::BACKGROUND.color());
    let label = state.layout.label;
    let mut buffer = Buffer::new(&mut state.fonts, Metrics::new(16.0, 19.0));
    buffer.set_size(Some(label.width()), Some(label.height()));
    buffer.set_text(MESSAGE, &Attrs::new(), Shaping::Advanced, None);
    buffer.shape_until_scroll(&mut state.fonts, false);
    let mut painter = Painter::new(canvas)
        .with_scale(scale)
        .with_text(&mut state.fonts, &mut state.cache);
    painter.text_buffer(&buffer, (label.x(), label.y()), chrome::FOREGROUND.color());
    for (button, bounds) in state.layout.buttons {
        controls::icon_button(
            &mut painter,
            bounds,
            button.label(),
            None,
            ButtonStyle {
                fill: chrome::HOVER,
                foreground: chrome::FOREGROUND,
                border: if state.hover == Some(button) || (state.focused && state.focus == button) {
                    chrome::ACCENT
                } else {
                    chrome::BORDER
                },
                font_size: 16.0,
                weight: Weight(500),
            },
        );
    }
}
