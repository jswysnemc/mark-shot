//! 【滤镜测试】【原图导出】验证本地图像、分数缩放捕获和裁切边缘都从冻结原图过滤

use super::filter_support::*;
use crate::editor::file_canvas::FileCanvas;
use crate::features::filter::FilterMode;
use tiny_skia::{Color, IntRect, Pixmap, Rect};

/// 【滤镜测试】【模式中心像素】返回原图颜色二百、一百、五十在 mode 与十四强度下的参考 RGBA
fn expected(mode: FilterMode) -> [u8; 4] {
    match mode {
        FilterMode::Pixelate | FilterMode::Blur => [200, 100, 50, 255],
        FilterMode::Grayscale => [126, 126, 126, 255],
        FilterMode::Invert => [55, 155, 205, 255],
        FilterMode::Brighten => [213, 136, 98, 255],
    }
}

/// 【滤镜测试】【原图分辨率】五模式导出保留一千六百乘九百原始尺寸，未修改像素与透明度不变
#[test]
fn native_file_exports_preserve_resolution_alpha_and_mode_pixels() {
    for mode in FilterMode::ALL {
        let mut dirty = 0;
        let mut state = filter_editor(&mut dirty);
        let mut original = Pixmap::new(1600, 900).unwrap();
        original.fill(Color::from_rgba8(0, 0, 255, 128));
        for y in 150..700 {
            for x in 100..1000 {
                let offset = (y * 1600 + x) * 4;
                original.data_mut()[offset..offset + 4].copy_from_slice(&[200, 100, 50, 255]);
            }
        }
        let file =
            FileCanvas::new(original, Rect::from_xywh(0.0, 0.0, 640.0, 480.0).unwrap()).unwrap();
        let viewport = file.viewport;
        let scale = file.original.width() as f32 / viewport.width();
        state.selection.zone = Some(viewport);
        state.file_canvas = Some(file);
        state.tool_settings.filter_mode = mode;
        draw(
            &mut state,
            (
                f64::from(viewport.left() + 100.0 / scale),
                f64::from(viewport.top() + 150.0 / scale),
            ),
            (
                f64::from(viewport.left() + 1000.0 / scale),
                f64::from(viewport.top() + 700.0 / scale),
            ),
            &mut dirty,
        );
        assert_eq!(filter(&state, 0).strength, 14.0);
        let bytes = crate::app::render_final(&mut state).unwrap().0;
        let output = image::load_from_memory(&bytes).unwrap().into_rgba8();
        assert_eq!(output.dimensions(), (1600, 900));
        assert_eq!(output.get_pixel(500, 400).0, expected(mode), "{mode:?}");
        for (x, y) in [(5, 5), (1599, 899), (1100, 720)] {
            assert_eq!(output.get_pixel(x, y).0, [0, 0, 255, 128]);
        }
    }
}

/// 【滤镜测试】【捕获分数缩放】一点二倍捕获保持原始导出尺寸，提亮强度不随导出比例变化
#[test]
fn fractional_capture_export_does_not_scale_brightness_strength() {
    let mut dirty = 0;
    let mut state = filter_editor(&mut dirty);
    let mut original = Pixmap::new(768, 576).unwrap();
    original.fill(Color::from_rgba8(200, 100, 50, 255));
    state.captures[0].pixmap = original;
    state.captures[0].scale = 1.2;
    state.placements[0].scale = 1.2;
    state.selection.zone = Rect::from_xywh(100.0, 60.0, 320.0, 220.0);
    state.tool_settings.filter_mode = FilterMode::Brighten;
    draw(&mut state, (150.0, 100.0), (250.0, 200.0), &mut dirty);
    let (bytes, origin, scale) = crate::app::render_final(&mut state).unwrap();
    let output = image::load_from_memory(&bytes).unwrap().into_rgba8();
    assert_eq!(output.dimensions(), (384, 264));
    assert_eq!(origin, (100, 60));
    assert_eq!(scale, 1.2);
    assert_eq!(output.get_pixel(120, 108).0, expected(FilterMode::Brighten));
    assert_eq!(output.get_pixel(0, 0).0, [200, 100, 50, 255]);
}

/// 【滤镜测试】【裁切外留量】选区裁切不截断模糊输入，局部导出逐像素等于完整导出的相同区域
#[test]
fn cropped_exports_match_the_same_pixels_of_the_full_export() {
    let mut dirty = 0;
    let mut state = filter_editor(&mut dirty);
    let source = &mut state.captures[0].pixmap;
    for y in 0..source.height() {
        for x in 0..source.width() {
            let offset = (y as usize * source.width() as usize + x as usize) * 4;
            source.data_mut()[offset..offset + 4].copy_from_slice(&[
                (x % 256) as u8,
                (y % 256) as u8,
                90,
                255,
            ]);
        }
    }
    state.tool_settings.filter_mode = FilterMode::Blur;
    draw(&mut state, (80.0, 100.0), (450.0, 300.0), &mut dirty);
    let whole = image::load_from_memory(&crate::app::render_final(&mut state).unwrap().0)
        .unwrap()
        .into_rgba8();
    let rect = IntRect::from_xywh(200, 120, 160, 120).unwrap();
    state.selection.zone = Some(rect.to_rect());
    let cropped = image::load_from_memory(&crate::app::render_final(&mut state).unwrap().0)
        .unwrap()
        .into_rgba8();
    let expected = image::imageops::crop_imm(&whole, 200, 120, 160, 120).to_image();
    assert_eq!(cropped.dimensions(), expected.dimensions());
    assert_eq!(
        cropped
            .as_raw()
            .iter()
            .zip(expected.as_raw())
            .position(|(actual, expected)| actual != expected),
        None
    );
}
