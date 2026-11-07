//! 【图片翻译】【共用绘制】先绘全部底板再绘正文，基线与排版共用字体，显示缩放不改原图坐标

use super::{TranslatedBlock, glyph_draw, shaping};
use cosmic_text::{FontSystem, SwashCache};
use tiny_skia::{FillRule, Paint, Pixmap, Transform};

/// 【图片翻译】【覆盖绘制】按 blocks、scale 在 canvas 绘制原版底板和基线，fonts、cache 只负责字形，不改变完整译文
pub fn draw(
    canvas: &mut Pixmap,
    blocks: &[TranslatedBlock],
    scale: f32,
    fonts: &mut FontSystem,
    cache: &mut SwashCache,
) {
    viewport(canvas, blocks, (0.0, 0.0), scale, fonts, cache);
}

/// 【图片翻译】【视区绘制】按 offset 像素偏移及 scale 直接绘入 canvas，fonts、cache 在显示比例栅格化字形，不分配原图大小覆盖层
pub fn viewport(
    canvas: &mut Pixmap,
    blocks: &[TranslatedBlock],
    offset: (f32, f32),
    scale: f32,
    fonts: &mut FontSystem,
    cache: &mut SwashCache,
) {
    viewport_xy(canvas, blocks, offset, (scale, scale), fonts, cache);
}

/// 【图片翻译】【双轴视区】按 offset 像素偏移及 scale 双轴比例绘入 canvas，贴图整数尺寸取整不改变行框对应的图像坐标
pub fn viewport_xy(
    canvas: &mut Pixmap,
    blocks: &[TranslatedBlock],
    offset: (f32, f32),
    scale: (f32, f32),
    fonts: &mut FontSystem,
    cache: &mut SwashCache,
) {
    if !scale.0.is_finite()
        || scale.0 <= 0.0
        || !scale.1.is_finite()
        || scale.1 <= 0.0
        || !offset.0.is_finite()
        || !offset.1.is_finite()
    {
        return;
    }
    let transform = Transform::from_scale(scale.0, scale.1).post_translate(offset.0, offset.1);
    for block in blocks {
        if let Some(rect) = block.plate.draw_rect()
            && let Some(path) = crate::renderer::rounded_rect_path(
                &rect,
                3.0_f32.min(rect.height() / 4.0),
                true,
                true,
                true,
                true,
            )
        {
            let mut paint = Paint::default();
            paint.set_color(block.background.into());
            paint.anti_alias = true;
            canvas.fill_path(&path, &paint, FillRule::Winding, transform, None);
        }
    }
    for block in blocks {
        shaping::prepare(fonts, &block.text);
        for line in &block.lines {
            for run in shaping::glyph_lines(
                fonts,
                &line.text,
                block.font_size,
                line.rect.height() as f32,
                None,
            ) {
                glyph_draw::draw(
                    canvas,
                    &run.glyphs,
                    (line.rect.left() as f32, line.baseline as f32),
                    offset,
                    scale,
                    block.foreground,
                    fonts,
                    cache,
                );
            }
        }
    }
}

/// 【图片翻译】【原图合成】将 blocks 绘入 image 同尺寸副本，fonts、cache 提供字体，返回原图分辨率图像
pub fn bake(
    image: &Pixmap,
    blocks: &[TranslatedBlock],
    fonts: &mut FontSystem,
    cache: &mut SwashCache,
) -> Pixmap {
    let mut result = image.clone();
    draw(&mut result, blocks, 1.0, fonts, cache);
    result
}
