//! 【尺子悬停】【临时说明】按主题绘制临时坐标与颜色说明，不用于导出

use super::{Painter, scale_rect, text::TextStyle};
use crate::features::ruler::labels::InfoLabel;
use cosmic_text::Weight;
use tiny_skia::{Pixmap, Rect};

/// 【尺子悬停】【说明正文】按 palette 将 label 的临时文字绘入 painter，支持窄范围多行，无返回值
pub(super) fn draw(
    painter: &mut Painter<'_>,
    label: &InfoLabel,
    palette: crate::theme::toolbar::Palette,
) {
    let origin = painter.origin();
    let size = painter.size();
    if Rect::from_xywh(origin.0, origin.1, size.0, size.1)
        .and_then(|visible| visible.intersect(&label.rect))
        .is_none()
    {
        return;
    }
    let scale = painter.scale();
    let Some(bounds) = scale_rect(&label.rect, scale).and_then(|rect| rect.round_out()) else {
        return;
    };
    let origin = (bounds.x() as f32 / scale, bounds.y() as f32 / scale);
    let Some(mut temporary) = Pixmap::new(bounds.width(), bounds.height()) else {
        return;
    };
    // 1. 【尺子】【说明正文】说明背景和字形使用固定像素原点，局部重绘不改变圆角采样
    {
        let mut local = painter.sub(&mut temporary).with_origin(origin);
        label_content(&mut local, label, palette);
    }
    let position = painter.to_px(origin.0, origin.1);
    painter.draw_pixmap(
        position.0.round() as i32,
        position.1.round() as i32,
        &temporary,
    );
}

/// 【尺子悬停】【说明内容】按 palette 将 label 背景与文字绘入 painter 的固定原点缓冲，无返回值
fn label_content(
    painter: &mut Painter<'_>,
    label: &InfoLabel,
    palette: crate::theme::toolbar::Palette,
) {
    let rect = label.rect;
    painter.fill_rounded(
        rect,
        (9.0 * label.unit).min(rect.height() / 2.0),
        palette.popup_fill.color(),
    );
    let padding = (10.0 * label.unit).min(rect.height() / 4.0);
    let height = (rect.height() - padding) / label.lines.len().max(1) as f32;
    for (index, line) in label.lines.iter().enumerate() {
        if let Some(line_rect) = Rect::from_xywh(
            rect.left(),
            rect.top() + padding / 2.0 + index as f32 * height,
            rect.width(),
            height,
        ) {
            painter.text(
                line_rect,
                line,
                TextStyle::new(label.font_size, palette.text)
                    .center()
                    .weight(Weight::SEMIBOLD),
            );
        }
    }
}

#[cfg(test)]
#[path = "ruler_hover_label_tests.rs"]
mod tests;
