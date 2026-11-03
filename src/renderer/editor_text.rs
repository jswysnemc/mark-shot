//! 【文字绘制】【共享编辑】标注文字与多行输入共用字形、选择和光标绘制

use cosmic_text::{Edit, Editor, SwashCache};

/// 【文字绘制】【编辑排版】使用 font_system、swash_cache 与 scale 绘制 editor；颜色数组分别控制正文、光标、选择和选中文字，f 接收物理像素区域
pub(crate) fn draw_editor(
    editor: &mut Editor<'static>,
    font_system: &mut cosmic_text::FontSystem,
    swash_cache: &mut SwashCache,
    scale: f32,
    [
        text_color,
        cursor_color,
        selection_color,
        selected_text_color,
    ]: [cosmic_text::Color; 4],
    mut f: impl FnMut(i32, i32, u32, u32, cosmic_text::Color),
) {
    editor.with_buffer_mut(|buffer| buffer.shape_until_scroll(font_system, false));
    let selection_bounds = editor.selection_bounds();
    let cursor = editor.cursor();
    editor.with_buffer(|buffer| {
        let width = buffer.size().0.unwrap_or(0.0) * scale;
        for run in buffer.layout_runs() {
            let line_i = run.line_i;
            let top = (run.line_top * scale) as i32;
            let height = (run.line_height * scale) as u32;

            if let Some((start, end)) = selection_bounds
                && line_i >= start.line
                && line_i <= end.line
            {
                let highlights: Vec<(f32, f32)> = run.highlight(start, end).collect();
                if highlights.is_empty() && run.glyphs.is_empty() && end.line > line_i {
                    f(0, top, width as i32 as u32, height, selection_color);
                } else {
                    let len = highlights.len();
                    for (idx, (x, w)) in highlights.into_iter().enumerate() {
                        let mut min = (x * scale) as i32;
                        let mut max = ((x + w) * scale) as i32;
                        if idx == len - 1 && end.line > line_i {
                            if run.rtl {
                                min = 0;
                            } else {
                                max = width as i32;
                            }
                        }
                        f(min, top, (max - min).max(0) as u32, height, selection_color);
                    }
                }
            }

            if let Some(x) = run.cursor_position(&cursor) {
                let w = scale.round().max(1.0) as u32;
                f((x * scale) as i32, top, w, height, cursor_color);
            }

            for glyph in run.glyphs {
                let physical = glyph.physical((0.0, run.line_y * scale), scale);
                let mut color = glyph.color_opt.unwrap_or(text_color);
                if text_color != selected_text_color
                    && let Some((start, end)) = selection_bounds
                    && line_i >= start.line
                    && line_i <= end.line
                    && (start.line != line_i || glyph.end > start.index)
                    && (end.line != line_i || glyph.start < end.index)
                {
                    color = selected_text_color;
                }
                swash_cache.with_pixels(font_system, physical.cache_key, color, |x, y, c| {
                    // 1. 【文字绘制】【字形透明度】Swash 的掩模像素不包含基础颜色 alpha，在字形回调中融合一次，不影响光标与选区
                    let alpha = (u32::from(c.a()) * u32::from(color.a()) + 127) / 255;
                    let pixel = cosmic_text::Color::rgba(c.r(), c.g(), c.b(), alpha as u8);
                    f(physical.x + x, physical.y + y, 1, 1, pixel);
                });
            }
            super::text_preedit::draw(&run, scale, text_color, &mut f);
        }
    });
}
