//! 【识别窗口】【统一框架】标题栏和内容留白使用逻辑像素，缩放后绘制与命中保持一致

use crate::{
    renderer::{Painter, text::TextStyle},
    ui::controls,
};
use tiny_skia::Rect;

pub(super) const HEADER: f32 = 32.0;
const MARGIN: f32 = 12.0;

pub(super) struct Layout {
    pub fit: f32,
    pub origin: (f32, f32),
    pub close: Rect,
}

/// 【识别窗口】【自然尺寸】按 panel 返回完整内容所需的窗口大小，无其他参数
pub(super) fn preferred(panel: (f32, f32)) -> (u32, u32) {
    (
        (panel.0 + MARGIN * 2.0).ceil() as u32,
        (panel.1 + HEADER + MARGIN * 2.0).ceil() as u32,
    )
}

/// 【识别窗口】【适配布局】按 size 和 panel 返回缩放、内容原点及关闭按钮，窗口受限时保留对称留白
pub(super) fn layout(size: (u32, u32), panel: (f32, f32)) -> Layout {
    let available = (
        (size.0 as f32 - MARGIN * 2.0).max(1.0),
        (size.1 as f32 - HEADER - MARGIN * 2.0).max(1.0),
    );
    let fit = (available.0 / panel.0)
        .min(available.1 / panel.1)
        .clamp(0.01, 1.0);
    Layout {
        fit,
        origin: (
            MARGIN + (available.0 - panel.0 * fit).max(0.0) * 0.5,
            HEADER + MARGIN,
        ),
        close: Rect::from_xywh((size.0 as f32 - 28.0).max(0.0), 4.0, 24.0, 24.0).unwrap(),
    }
}

/// 【识别窗口】【标题绘制】按 painter、size、hovered 和 pressed 绘制统一关闭按钮与紧凑标题，无返回值
pub(super) fn draw(painter: &mut Painter<'_>, size: (u32, u32), hovered: bool, pressed: bool) {
    let close = layout(size, (1.0, 1.0)).close;
    painter.text(
        Rect::from_xywh(12.0, 0.0, (size.0 as f32 - 52.0).max(1.0), HEADER).unwrap(),
        "Mark Shot / OCR",
        TextStyle::new(12.0, crate::theme::window::Palette::current().text),
    );
    if hovered || pressed {
        controls::button(painter, close, "", pressed, hovered, true);
    }
    let color = if pressed {
        crate::theme::color::on_active()
    } else if hovered {
        crate::theme::toolbar::Palette::current().hover_text
    } else {
        crate::theme::window::Palette::current().text
    };
    painter.icon(
        crate::ui::icons::CLOSE,
        14.0,
        close.left() + 5.0,
        close.top() + 5.0,
        color,
    );
}
