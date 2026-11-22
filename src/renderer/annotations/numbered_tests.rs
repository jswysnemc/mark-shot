//! 【编号标注测试】【栅格隔离】验证实际编号正文使用标注缓存，界面 Fontconfig 渲染不进入导出

use super::*;
use cosmic_text::{CacheKey, CacheKeyFlags, SwashCache, Weight};

/// 【编号标注测试】【真实入口】无参数，以已映射系统字体绘制编号箭头，验证正文使用 Swash 缓存而非界面 FreeType，无返回值
#[test]
fn numbered_arrow_keeps_annotation_rasterization() {
    // 1. 【编号标注测试】【目标字形】通过实际编号排版找到数字，确保界面入口能够使用共享字体映射
    let mut fonts = crate::fonts::ui_system::new();
    let config = &crate::config::get().annotations.numbered;
    let stroke = 5.0;
    let size = stroke * config.circle * config.digit_sizes[0];
    let weight = if config.bold {
        Weight::BOLD
    } else {
        Weight::NORMAL
    };
    let (buffer, _, _) =
        shape_single_line(&mut fonts, "7", size, weight, cosmic_text::Style::Normal);
    let glyph = &buffer.layout_runs().next().unwrap().glyphs[0];
    let key = CacheKey::new(
        glyph.font_id,
        glyph.glyph_id,
        size,
        (0.0, 0.0),
        weight,
        CacheKeyFlags::empty(),
    )
    .0;
    assert!(crate::fonts::ui_raster::with_image(
        &fonts,
        key,
        true,
        |_| {}
    ));

    // 2. 【编号标注测试】【正文绘制】调用生产编号入口，不以替身字形构造预期
    let mut canvas = Pixmap::new(180, 100).unwrap();
    let mut cache = SwashCache::new();
    numbered::draw_numerated_arrow(
        &mut Painter::new(&mut canvas).with_text(&mut fonts, &mut cache),
        (45.0, 50.0),
        (140.0, 50.0),
        7,
        Color::from_rgba8(80, 100, 180, 255),
        stroke,
        Color::TRANSPARENT,
    );

    // 3. 【编号标注测试】【路径断言】正文必须进入独立标注缓存，界面入口会使此缓存保持为空
    assert!(cache.image_cache.values().any(|image| image.is_some()));
}
