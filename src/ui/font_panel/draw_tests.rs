//! 【字体主题测试】【实际像素】注入明暗调色板，验证文字可读性、控件亮度与同面板恢复

use super::{FontPanel, draw, layout::FontLayout};
use crate::{
    fonts::font_catalog::{Family, FontCatalog},
    renderer::Painter,
    theme::{Mode, toolbar::Palette},
};
use cosmic_text::SwashCache;
use tiny_skia::{Pixmap, Rect};

/// 【字体主题测试】【固定目录】返回包含选中字体和可滚动字体的面板，不访问系统目录
fn panel() -> FontPanel {
    let families = (0..20)
        .map(|index| Family {
            name: if index == 0 {
                "Inter".into()
            } else {
                format!("Sample Font {index}")
            },
            faces: Vec::new(),
        })
        .collect();
    FontPanel {
        open: true,
        layout: FontLayout::new(20, (640.0, 480.0)),
        family: "Inter".into(),
        catalog: Some(FontCatalog { families }),
        filtered: (0..20).collect(),
        ..Default::default()
    }
}

/// 【字体主题测试】【面板绘制】按 mode、scale 绘制 panel 的真实内容，返回固定背景上的像素
fn render(panel: &mut FontPanel, mode: Mode, scale: f32) -> Pixmap {
    let mut image = Pixmap::new(
        (panel.layout.size.0 * scale).ceil() as u32,
        (panel.layout.size.1 * scale).ceil() as u32,
    )
    .unwrap();
    image.fill(tiny_skia::Color::from_rgba8(80, 85, 90, 255));
    let mut fonts = crate::fonts::bundled();
    let mut cache = SwashCache::new();
    draw::content(
        &mut Painter::new(&mut image)
            .with_scale(scale)
            .with_text(&mut fonts, &mut cache),
        panel,
        Palette::new(mode),
    );
    image
}

/// 【字体主题测试】【亮度读取】按 image、scale、point 返回实际像素的平均通道亮度
fn brightness(image: &Pixmap, scale: f32, point: (f32, f32)) -> u16 {
    let pixel = image
        .pixel((point.0 * scale) as u32, (point.1 * scale) as u32)
        .unwrap();
    (pixel.red() as u16 + pixel.green() as u16 + pixel.blue() as u16) / 3
}

/// 【字体主题测试】【字形统计】按 image、rect、scale 和 light 统计具有清晰明暗差异的正文像素，返回数量
fn readable(image: &Pixmap, rect: Rect, scale: f32, light: bool) -> usize {
    let mut count = 0;
    for y in (rect.top() * scale) as u32..(rect.bottom() * scale) as u32 {
        for x in (rect.left() * scale) as u32..(rect.right() * scale) as u32 {
            let pixel = image.pixel(x, y).unwrap();
            let luminance = (pixel.red() as u16 + pixel.green() as u16 + pixel.blue() as u16) / 3;
            count += usize::from(if light {
                luminance < 130
            } else {
                luminance > 150
            });
        }
    }
    count
}

/// 【字体主题测试】【浅色字体】无参数，正常字体在浅色列表上有清晰暗色字形，分数比例同样可读，无返回值
#[test]
fn light_font_preview_is_readable_on_the_actual_popup() {
    for scale in [1.0, 1.25, 2.0] {
        let mut panel = panel();
        let row = panel.rows()[1].1;
        let image = render(&mut panel, Mode::Light, scale);
        let text = Rect::from_xywh(row.left() + 34.0, row.top() + 4.0, 180.0, 20.0).unwrap();
        assert!(readable(&image, text, scale, true) > (50.0 * scale * scale) as usize);
    }
}

/// 【字体主题测试】【深色控件】无参数，字号与样式底板跟随深色，字体预览仍保留亮色字形，无返回值
#[test]
fn dark_font_controls_do_not_keep_white_native_style_backgrounds() {
    let mut panel = panel();
    let image = render(&mut panel, Mode::Dark, 1.0);
    for cell in [
        panel.layout.size_field,
        panel.layout.bold,
        panel.layout.italic,
    ] {
        assert!(brightness(&image, 1.0, (cell.left() + 5.0, cell.top() + 4.0)) < 100);
    }
    let row = panel.rows()[1].1;
    assert!(
        readable(
            &image,
            Rect::from_xywh(row.left() + 34.0, row.top() + 4.0, 180.0, 20.0).unwrap(),
            1.0,
            false
        ) > 50
    );
}

/// 【字体主题测试】【滚动轨道】无参数，轨道与手柄具有清晰亮度差异，浅色轨道不保留黑色，无返回值
#[test]
fn light_scrollbar_track_and_thumb_have_visible_contrast() {
    let mut panel = panel();
    let image = render(&mut panel, Mode::Light, 1.0);
    let thumb = panel.thumb().unwrap();
    let x = thumb.left() + 7.0;
    let track = brightness(&image, 1.0, (x, thumb.bottom() + 8.0));
    let handle = brightness(&image, 1.0, (x, thumb.top() + thumb.height() / 2.0));
    assert!(track > 200, "light track remained dark: {track}");
    assert!(
        track.abs_diff(handle) > 45,
        "scrollbar lost contrast: {track}, {handle}"
    );
}

/// 【字体主题测试】【同状态恢复】无参数，明暗往返保持全部像素、目录缓存及已选字体，列表偏移不重置，无返回值
#[test]
fn font_panel_roundtrip_preserves_pixels_and_visible_font_cache() {
    let mut panel = panel();
    panel.offset = 28.0;
    let light = render(&mut panel, Mode::Light, 1.25);
    let key = panel.preview.key.clone();
    let dark = render(&mut panel, Mode::Dark, 1.25);
    assert_ne!(light.data(), dark.data());
    let restored = render(&mut panel, Mode::Light, 1.25);
    assert_eq!(light.data(), restored.data());
    assert_eq!(panel.preview.key, key);
    assert_eq!(panel.offset, 28.0);
    assert_eq!(panel.family, "Inter");
}
