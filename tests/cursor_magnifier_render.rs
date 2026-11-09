//! 【光标放大区测试】【原图读数】验证边缘像素居中、实际色块与分数缩放下的完整绘制范围

use cosmic_text::SwashCache;
use mark_shot::{
    renderer::Painter,
    types::Capture,
    ui::magnifier::{draw_magnifier, magnifier_rect, sample_pixel, size},
};
use tiny_skia::{Color, Pixmap, Rect};

/// 【光标放大区测试】【原图构造】按 dimensions、point 和 scale 创建仅指针像素为红色的捕获，返回原图与比例
fn capture(dimensions: (u32, u32), point: (u32, u32), scale: f32) -> Capture {
    let mut pixels = Pixmap::new(dimensions.0, dimensions.1).unwrap();
    pixels.fill(Color::from_rgba8(20, 40, 80, 255));
    pixels.pixels_mut()[(point.1 * dimensions.0 + point.0) as usize] =
        Color::from_rgba8(220, 60, 90, 255)
            .premultiply()
            .to_color_u8();
    Capture {
        pixmap: pixels.into(),
        scale,
    }
}

/// 【光标放大区测试】【真实绘制】按 capture 和 cursor 绘制到分数缩放画布，返回画布及逻辑范围
fn render(capture: &Capture, cursor: (f32, f32)) -> (Pixmap, Rect) {
    let scale = capture.scale;
    let mut pixels = Pixmap::new((400.0 * scale) as u32, (300.0 * scale) as u32).unwrap();
    let mut fonts = mark_shot::fonts::bundled();
    let mut cache = SwashCache::new();
    draw_magnifier(
        &mut Painter::new(&mut pixels)
            .with_scale(scale)
            .with_text(&mut fonts, &mut cache),
        capture,
        cursor,
        (400.0, 300.0),
    );
    (pixels, magnifier_rect(cursor, 400.0, 300.0))
}

/// 【光标放大区测试】【指针对齐】无参数，屏幕角落和小于采样网格的图像仍保持中心像素与颜色值一致
#[test]
fn cursor_magnifier_centers_the_sample_at_edges_and_on_tiny_images() {
    for scale in [1.0, 1.25, 1.5, 2.0] {
        let center = capture((64, 48), (32, 24), scale);
        let (baseline, bounds) = render(&center, (32.25 / scale, 24.25 / scale));
        let center_pixel = baseline
            .pixel(
                ((bounds.left() + bounds.width() / 2.0) * scale).round() as u32,
                ((bounds.top() + size() as f32 / 2.0) * scale).round() as u32,
            )
            .unwrap();
        for (dimensions, point) in [((64, 48), (0, 0)), ((64, 48), (63, 47)), ((2, 1), (1, 0))] {
            let capture = capture(dimensions, point, scale);
            let cursor = (
                (point.0 as f32 + 0.25) / scale,
                (point.1 as f32 + 0.25) / scale,
            );
            let native = capture.to_native((f64::from(cursor.0), f64::from(cursor.1)));
            assert_eq!(
                sample_pixel(&capture.pixmap, native),
                Some(Color::from_rgba8(220, 60, 90, 255))
            );
            let (pixels, bounds) = render(&capture, cursor);
            assert_eq!(
                pixels.pixel(
                    ((bounds.left() + bounds.width() / 2.0) * scale).round() as u32,
                    ((bounds.top() + size() as f32 / 2.0) * scale).round() as u32,
                ),
                Some(center_pixel),
                "scale={scale}, dimensions={dimensions:?}, point={point:?}"
            );

            let label_top = ((bounds.top() + size() as f32) * scale).ceil() as u32;
            let swatch_pixels = pixels.pixels()[(label_top * pixels.width()) as usize..]
                .iter()
                .filter(|pixel| {
                    **pixel
                        == Color::from_rgba8(220, 60, 90, 255)
                            .premultiply()
                            .to_color_u8()
                })
                .count();
            assert!(
                swatch_pixels > 20,
                "readout omitted the actual source color"
            );
        }
    }
}

/// 【光标放大区测试】【屏幕避让】无参数，四角读数保持在屏幕内，整体损坏范围包含圆镜与颜色坐标条
#[test]
fn cursor_magnifier_damage_contains_all_pixels_at_screen_corners() {
    for scale in [1.0, 1.25, 1.5, 2.0] {
        let capture = capture(
            ((400.0 * scale) as u32, (300.0 * scale) as u32),
            (0, 0),
            scale,
        );
        for cursor in [(0.0, 0.0), (399.0, 0.0), (0.0, 299.0), (399.0, 299.0)] {
            let (pixels, bounds) = render(&capture, cursor);
            assert!(
                bounds.left() >= 0.0
                    && bounds.top() >= 0.0
                    && bounds.right() <= 400.0
                    && bounds.bottom() <= 300.0
            );
            for (index, pixel) in pixels
                .pixels()
                .iter()
                .enumerate()
                .filter(|(_, pixel)| pixel.alpha() != 0)
            {
                let horizontal = (index as u32 % pixels.width()) as f32 / scale;
                let vertical = (index as u32 / pixels.width()) as f32 / scale;
                assert!(
                    horizontal >= bounds.left() - 2.0
                        && horizontal < bounds.right() + 2.0
                        && vertical >= bounds.top() - 2.0
                        && vertical < bounds.bottom() + 2.0,
                    "pixel={pixel:?}, bounds={bounds:?}"
                );
            }
        }
    }
}
