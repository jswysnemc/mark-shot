//! 【取色标注】【像素绘制】点位、引线、色块与固定颜色标签共同进入标注导出

use super::ColorSample;
use crate::{
    renderer::{Painter, text::TextStyle},
    theme::Rgba,
};

/// 【取色标注】【正文绘制】将 sample 绘入 painter，颜色来自取样快照，窗口缩放不重新取色，无返回值
pub fn draw(painter: &mut Painter<'_>, sample: ColorSample) {
    let bounds = sample.label_rect();
    let mut path = tiny_skia::PathBuilder::new();
    path.move_to(sample.source.0, sample.source.1);
    path.line_to(
        sample.source.0.clamp(bounds.left(), bounds.right()),
        bounds.top() + bounds.height() / 2.0,
    );
    let foreground = Rgba(235, 239, 245, 255);
    painter.stroke_path(
        &path.finish().unwrap(),
        foreground.color(),
        &tiny_skia::Stroke {
            width: (sample.size / 10.0).max(1.0),
            ..Default::default()
        },
    );
    crate::ui::controls::frame(
        painter,
        bounds,
        Rgba(35, 40, 47, 250),
        Rgba(115, 128, 144, 255),
        sample.size * 0.35,
    );
    let dot =
        tiny_skia::Rect::from_xywh(sample.source.0 - 3.0, sample.source.1 - 3.0, 6.0, 6.0).unwrap();
    painter.fill_rounded(dot, 3.0, foreground);
    let swatch = tiny_skia::Rect::from_xywh(
        bounds.x() + sample.size * 0.5,
        bounds.y() + sample.size * 0.5,
        sample.size * 1.2,
        sample.size * 1.2,
    )
    .unwrap();
    painter.fill_rounded(swatch, sample.size * 0.2, sample.color);
    let text = crate::ui::settings_panel::ValueField::Hex.text(sample.color);
    painter.text(
        tiny_skia::Rect::from_xywh(
            bounds.x() + sample.size * 2.2,
            bounds.y(),
            bounds.width() - sample.size * 2.5,
            bounds.height(),
        )
        .unwrap(),
        &text,
        TextStyle::new(sample.size, foreground),
    );
}
