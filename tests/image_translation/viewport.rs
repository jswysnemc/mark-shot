//! 【图片翻译测试】【显示视区】直接缩放绘制与整图裁切保持相同场景坐标，所有底板先于正文

use mark_shot::{
    features::image_translation::{BoxRect, TranslatedBlock, TranslatedLine, paint},
    theme::Rgba,
};
use tiny_skia::Pixmap;

/// 【图片翻译测试】【覆盖样本】返回可跨底板重叠的独立段落，无参数，不调用 OCR 或翻译网络
fn blocks() -> Vec<TranslatedBlock> {
    vec![
        TranslatedBlock {
            source: BoxRect::new(10.0, 10.0, 70.0, 20.0),
            plate: BoxRect::new(8.0, 8.0, 74.0, 24.0),
            background: Rgba(250, 250, 250, 255),
            foreground: Rgba(10, 20, 30, 255),
            font_size: 14.0,
            text: "Hello World".into(),
            lines: vec![TranslatedLine {
                text: "Hello World".into(),
                rect: BoxRect::new(10.0, 10.0, 80.0, 20.0),
                baseline: 25.0,
            }],
        },
        TranslatedBlock {
            source: BoxRect::new(40.0, 10.0, 70.0, 20.0),
            plate: BoxRect::new(38.0, 8.0, 74.0, 24.0),
            background: Rgba(230, 240, 250, 255),
            foreground: Rgba(20, 30, 40, 255),
            font_size: 14.0,
            text: String::new(),
            lines: Vec::new(),
        },
    ]
}

/// 【图片翻译测试】【视区裁切】整数像素偏移下局部直接绘制与整图结果逐像素一致，二倍比例也不改变字形基线
#[test]
fn direct_viewport_matches_full_size_crop_at_native_and_double_scale() {
    let mut fonts = mark_shot::fonts::bundled();
    let mut cache = cosmic_text::SwashCache::new();
    for scale in [1.0, 2.0] {
        let mut full = Pixmap::new((130.0 * scale) as u32, (60.0 * scale) as u32).unwrap();
        paint::draw(&mut full, &blocks(), scale, &mut fonts, &mut cache);
        let mut crop = Pixmap::new((50.0 * scale) as u32, (30.0 * scale) as u32).unwrap();
        let (x, y) = ((30.0 * scale) as u32, (5.0 * scale) as u32);
        paint::viewport(
            &mut crop,
            &blocks(),
            (-(x as f32), -(y as f32)),
            scale,
            &mut fonts,
            &mut cache,
        );
        for row in 0..crop.height() {
            for col in 0..crop.width() {
                assert_eq!(
                    crop.pixel(col, row),
                    full.pixel(col + x, row + y),
                    "scale {scale}, pixel {col},{row}"
                );
            }
        }
    }
}

/// 【图片翻译测试】【原图保护】合成返回独立图像，正文仍可绘在后续段落底板上，原图不受影响
#[test]
fn plates_precede_all_text_and_baking_preserves_the_original_image() {
    let mut fonts = mark_shot::fonts::bundled();
    let mut cache = cosmic_text::SwashCache::new();
    let image = Pixmap::new(130, 60).unwrap();
    let baked = paint::bake(&image, &blocks(), &mut fonts, &mut cache);
    assert!(image.data().iter().all(|value| *value == 0));
    assert!((40..80).any(|x| (12..26).any(|y| baked.pixel(x, y).unwrap().red() < 100)));
}
