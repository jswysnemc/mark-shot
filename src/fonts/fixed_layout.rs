//! 【界面字体】【定点布局】在换行和绘制前量化字形前进量，保持原版六十四分之一像素精度

use cosmic_text::{Buffer, FontSystem, Hinting, LayoutLine, Wrap};

/// 【界面字体】【字形量化】按 fonts、buffer、size、width 和 wrap 返回定点布局，保留双向文字簇与字形装饰
pub(crate) fn lines(
    fonts: &mut FontSystem,
    buffer: &Buffer,
    size: f32,
    width: Option<f32>,
    wrap: Wrap,
) -> Vec<LayoutLine> {
    let mut result = Vec::new();
    for line in &buffer.lines {
        let Some(mut shaped) = line.shape_opt().cloned() else {
            continue;
        };
        for span in &mut shaped.spans {
            for word in &mut span.words {
                for glyph in &mut word.glyphs {
                    // 1. 【界面字体】【前进量】字体基础步进截断，整形产生的字距调整按同一固定精度保存
                    let pixels = f64::from(glyph.x_advance) * f64::from(size) * 64.0;
                    let base = fonts
                        .get_font(glyph.font_id, glyph.font_weight)
                        .map(|font| {
                            f64::from(
                                font.as_swash()
                                    .glyph_metrics(&[])
                                    .scale(size)
                                    .advance_width(glyph.glyph_id),
                            ) * 64.0
                        });
                    let fixed = base.map_or_else(
                        || (pixels + 0.0001).floor(),
                        |base| (base + 0.0001).floor() + (pixels - base).round(),
                    );
                    glyph.x_advance = (fixed / 64.0 / f64::from(size)) as f32;
                }
            }
        }
        let mut layouts = shaped.layout(size, width, wrap, None, None, Hinting::Disabled);
        if wrap != Wrap::None {
            let count = layouts.len();
            for layout in layouts.iter_mut().take(count.saturating_sub(1)) {
                // 2. 【界面字体】【软换行空格】原版不把自动换行前的空格计入自然宽度，正文和显式行尾空格仍保留
                let end = layout
                    .glyphs
                    .iter()
                    .map(|glyph| glyph.end)
                    .max()
                    .unwrap_or(0);
                let visible_end = line.text()[..end].trim_end().len();
                layout.w = layout
                    .glyphs
                    .iter()
                    .filter(|glyph| glyph.start < visible_end)
                    .map(|glyph| glyph.x + glyph.w)
                    .fold(0.0, f32::max);
            }
        }
        // 3. 【界面字体】【浮点往返】定点字宽转换为设计单位后再布局会产生邻近误差，恢复已经确定的六十四分之一像素位置
        let fixed = |value: f32| (value * 64.0).round() / 64.0;
        for layout in &mut layouts {
            layout.w = fixed(layout.w);
            for glyph in &mut layout.glyphs {
                glyph.x = fixed(glyph.x);
                glyph.w = fixed(glyph.w);
            }
        }
        result.extend(layouts);
    }
    result
}
