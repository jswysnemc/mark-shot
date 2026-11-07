//! 【选区尺寸】【输入度量】定点字形、末端悬垂和真实选择共用同一布局

use crate::{renderer::line_edit, ui::text_field::LineEditState};
use cosmic_text::{CacheKey, CacheKeyFlags, Command, FontSystem, LayoutGlyph, SwashCache, Wrap};

/// 【选区尺寸】【字形边界】按 glyph、fonts 与 cache 返回未做像素微调的紧密轮廓范围，缺失字形返回空值
fn bounds(
    glyph: &LayoutGlyph,
    fonts: &mut FontSystem,
    cache: &mut SwashCache,
) -> Option<tiny_skia::Rect> {
    let (key, _, _) = CacheKey::new(
        glyph.font_id,
        glyph.glyph_id,
        glyph.font_size,
        (0.0, 0.0),
        glyph.font_weight,
        glyph.cache_key_flags | CacheKeyFlags::DISABLE_HINTING,
    );
    let commands = cache.get_outline_commands(fonts, key)?;
    let mut path = tiny_skia::PathBuilder::new();
    for command in commands {
        match command {
            Command::MoveTo(p) => path.move_to(p.x, p.y),
            Command::LineTo(p) => path.line_to(p.x, p.y),
            Command::QuadTo(c, p) => path.quad_to(c.x, c.y, p.x, p.y),
            Command::CurveTo(a, b, p) => path.cubic_to(a.x, a.y, b.x, b.y, p.x, p.y),
            Command::Close => path.close(),
        }
    }
    path.finish()?.compute_tight_bounds()
}

/// 【选区尺寸】【定点字段】按 text、field、fonts 和 cache 替换字形布局，返回包含末字悬垂的自然宽度，不取整前进总量
pub(super) fn fixed(
    text: &mut line_edit::Shape,
    field: &LineEditState,
    fonts: &mut FontSystem,
    cache: &mut SwashCache,
) -> f32 {
    // 1. 【选区尺寸】【字形步进】渲染、光标和选择共用量化后的完整字形布局
    let mut layouts = crate::fonts::fixed_layout::lines(
        fonts,
        &text.buffer,
        super::layout::FONT_SIZE,
        None,
        Wrap::None,
    );
    let mut natural = 0.0_f32;
    for line in &mut layouts {
        let fixed = |value: f32| (value * 64.0).floor() / 64.0;
        line.max_ascent = fixed(line.max_ascent);
        line.max_descent = fixed(line.max_descent);
        line.line_height_opt = Some((line.max_ascent + line.max_descent).ceil().max(1.0));
        let mut width = line.w;
        // 2. 【选区尺寸】【末字悬垂】原版自然宽度包含末字负右边距，居中与滚动不能只使用前进总量
        if let Some(glyph) = line
            .glyphs
            .iter()
            .max_by(|a, b| (a.x + a.w).total_cmp(&(b.x + b.w)))
            && let Some(bounds) = bounds(glyph, fonts, cache)
        {
            let bearing =
                glyph.w.round() - (glyph.font_size * glyph.x_offset + bounds.right()).ceil();
            width = width.max(glyph.x + glyph.w - bearing.min(0.0));
        }
        natural = natural.max(width);
    }
    text.use_layout(layouts, field);
    natural
}
