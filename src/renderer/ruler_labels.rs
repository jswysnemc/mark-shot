//! 【尺子说明渲染】【固定绘制】完整标签共用原版尺子字体和固定像素网格，局部重绘不改变采样

use super::Painter;
use crate::features::ruler::labels::InfoLabel;
use cosmic_text::LayoutRun;
use tiny_skia::{Color, Rect};

/// 【尺子说明渲染】【完整标签】按painter和label绘制原版背景与居中正文，单个临时网格不超过128乘128像素，无返回值
fn label(painter: &mut Painter<'_>, label: &InfoLabel) {
    let window = painter.window_units();
    let pixels = window.window(label.font_size).round().max(6.0);
    let factor = window.canvas(1.0);
    let Some(fonts) = painter.fonts.as_mut() else {
        return;
    };
    let shapes: Vec<_> = label
        .lines
        .iter()
        .map(|text| {
            let (buffer, mut layout) = crate::fonts::ruler::shape(fonts.system, text, pixels);
            let metrics = crate::fonts::ruler::measure(text, pixels);
            for glyph in &mut layout.glyphs {
                glyph.x *= factor;
                glyph.y *= factor;
                glyph.w *= factor;
                glyph.font_size *= factor;
            }
            (buffer, layout, metrics)
        })
        .collect();
    // 1. 【尺子说明渲染】【固定栅格】完整标签尺寸保持原版值，只分配与当前视口相交的小块
    super::rotation_tiles::draw_region(painter, label.rect, |painter| {
        painter.fill_rounded(
            label.rect,
            9.0 * label.unit,
            Color::from_rgba8(8, 13, 19, 225),
        );
        let height = label.rect.height() / shapes.len().max(1) as f32;
        for (index, (buffer, layout, metrics)) in shapes.iter().enumerate() {
            let Some(raw) = buffer.layout_runs().next() else {
                continue;
            };
            let run = LayoutRun {
                glyphs: &layout.glyphs,
                decorations: &layout.decorations,
                line_y: metrics.ascent * factor,
                line_w: metrics.width * factor,
                line_height: metrics.height * factor,
                ..raw
            };
            let position = (
                label.rect.left() + (label.rect.width() - run.line_w) / 2.0,
                label.rect.top() + index as f32 * height + (height - run.line_height) / 2.0,
            );
            painter.text_layout_run(&run, position, Color::from_rgba8(229, 231, 235, 255));
        }
    });
}

/// 【尺子说明渲染】【说明集合】按labels顺序绘入painter，角度与长度顺序由布局模块决定，无返回值
pub(crate) fn draw(painter: &mut Painter<'_>, labels: &[InfoLabel]) {
    let origin = painter.origin();
    let size = painter.size();
    let Some(visible) = Rect::from_xywh(origin.0, origin.1, size.0, size.1) else {
        return;
    };
    for item in labels
        .iter()
        .filter(|item| item.rect.intersect(&visible).is_some())
    {
        label(painter, item);
    }
}
