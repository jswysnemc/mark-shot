//! 【尺子悬停测试】【主题像素】验证临时提示可读性及明暗恢复，不修改全局主题

use super::*;
use crate::theme::{Mode, toolbar::Palette};
use tiny_skia::Color;

/// 【尺子悬停测试】【固定提示】按mode和scale绘制真实悬停内容，返回确定背景上的提示像素
fn render(mode: Mode, scale: f32) -> Pixmap {
    let mut image = Pixmap::new((240.0 * scale) as u32, (100.0 * scale) as u32).unwrap();
    image.fill(Color::from_rgba8(90, 120, 150, 255));
    let mut fonts = crate::fonts::bundled();
    let mut cache = cosmic_text::SwashCache::new();
    draw(
        &mut Painter::new(&mut image)
            .with_scale(scale)
            .with_text(&mut fonts, &mut cache),
        &InfoLabel {
            rect: Rect::from_xywh(20.0, 20.0, 190.0, 60.0).unwrap(),
            unit: 1.0,
            lines: vec!["X: 511  Y: 392".into(), "RGB: 90, 120, 150".into()],
            font_size: 13.0,
        },
        Palette::new(mode),
    );
    image
}

/// 【尺子悬停测试】【默认浅色】无参数，浅色提示底板具有高亮度且正文存在清晰暗色字形，各设备比例一致，无返回值
#[test]
fn light_ruler_hover_label_has_light_background_and_dark_text() {
    for scale in [1.0, 1.25, 2.0] {
        let image = render(Mode::Light, scale);
        let background = image
            .pixel((30.0 * scale) as u32, (30.0 * scale) as u32)
            .unwrap();
        assert!(background.red() > 210 && background.green() > 210 && background.blue() > 210);
        let dark_pixels = image
            .pixels()
            .iter()
            .filter(|pixel| pixel.red() < 120 && pixel.green() < 120 && pixel.blue() < 120)
            .count();
        assert!(dark_pixels > (100.0 * scale * scale) as usize);
    }
}

/// 【尺子悬停测试】【深色恢复】无参数，深色底板和文字仍可读，明暗往返保持完整像素，无返回值
#[test]
fn ruler_hover_label_dark_theme_and_roundtrip_remain_readable() {
    let light = render(Mode::Light, 1.25);
    let dark = render(Mode::Dark, 1.25);
    assert!(
        light.data() != dark.data(),
        "ruler hover retained the same theme pixels"
    );
    let pixel = dark.pixel(38, 38).unwrap();
    assert!(pixel.red() < 100 && pixel.green() < 100 && pixel.blue() < 100);
    assert!(
        dark.pixels()
            .iter()
            .filter(|pixel| pixel.red() > 160 && pixel.green() > 160 && pixel.blue() > 160)
            .count()
            > 100
    );
    assert!(
        light.data() == render(Mode::Light, 1.25).data(),
        "ruler hover roundtrip changed pixels"
    );
}
