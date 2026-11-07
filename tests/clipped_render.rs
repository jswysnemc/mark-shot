//! 【局部绘制测试】【缓冲范围】全画布损坏复用原缓冲，部分损坏仍隔离动态像素

use mark_shot::renderer::Painter;
use tiny_skia::{Color, Pixmap, Rect};

/// 【局部绘制测试】【整屏复用】完整 dirty 像素范围不再分配第二份画布；覆盖多种 scale 与偏移 origin，无参数、无返回值
#[test]
fn full_canvas_damage_draws_into_the_original_buffer() {
    for scale in [0.75, 1.0, 1.2, 2.0] {
        let mut canvas = Pixmap::new(128, 80).unwrap();
        let original = canvas.data().as_ptr();
        let dirty = Rect::from_xywh(0.0, 0.0, 128.0 / scale, 80.0 / scale).unwrap();
        let mut called = false;
        Painter::new(&mut canvas)
            .with_scale(scale)
            .with_origin((10.25, -5.125))
            .clipped(Some(dirty), |painter| {
                called = true;
                assert_eq!(painter.canvas().data().as_ptr(), original, "scale={scale}");
                assert_eq!(painter.origin(), (10.25, -5.125));
                painter.canvas().fill(Color::from_rgba8(255, 77, 77, 128));
            });
        assert!(called);
        assert!(canvas.pixels().iter().all(|pixel| pixel.alpha() == 128));
    }
}

/// 【局部绘制测试】【超界整屏】扩展 dirty 经画布约束后覆盖整屏也复用原缓冲，无参数、无返回值
#[test]
fn oversized_damage_reuses_the_original_buffer_after_canvas_clamping() {
    let mut canvas = Pixmap::new(128, 80).unwrap();
    let original = canvas.data().as_ptr();
    Painter::new(&mut canvas).clipped(
        Some(Rect::from_xywh(-8.0, -10.0, 200.0, 160.0).unwrap()),
        |painter| assert_eq!(painter.canvas().data().as_ptr(), original),
    );
}

/// 【局部绘制测试】【部分隔离】窄 dirty 使用独立局部缓冲，只覆盖实际损坏像素，保留外部背景，无参数、无返回值
#[test]
fn partial_damage_keeps_an_isolated_buffer_and_preserves_outside_pixels() {
    let mut canvas = Pixmap::new(128, 80).unwrap();
    let background = Color::from_rgba8(30, 34, 38, 255);
    canvas.fill(background);
    let original = canvas.data().as_ptr();
    let untouched = canvas.pixel(0, 0).unwrap();
    Painter::new(&mut canvas).clipped(
        Some(Rect::from_xywh(10.0, 15.0, 2.0, 30.0).unwrap()),
        |painter| {
            assert_ne!(painter.canvas().data().as_ptr(), original);
            assert_eq!(painter.canvas().width(), 2);
            assert_eq!(painter.canvas().height(), 30);
            assert!(
                painter
                    .canvas()
                    .pixels()
                    .iter()
                    .all(|pixel| *pixel == untouched)
            );
            painter.canvas().fill(Color::from_rgba8(255, 77, 77, 128));
        },
    );
    for y in 0..80 {
        for x in 0..128 {
            let pixel = canvas.pixel(x, y).unwrap();
            if (10..12).contains(&x) && (15..45).contains(&y) {
                assert_eq!(pixel.alpha(), 128);
            } else {
                assert_eq!(pixel, untouched);
            }
        }
    }
}

/// 【局部绘制测试】【嵌套场景坐标】两层场景裁切与直接裁切逐像素一致，同时保留既有本地坐标接口，无参数、无返回值
#[test]
fn nested_scene_clipping_matches_direct_damage_at_fractional_scales() {
    for scale in [1.0, 1.25, 2.0] {
        let mut direct = Pixmap::new(160, 100).unwrap();
        direct.fill(Color::from_rgba8(30, 34, 38, 255));
        let mut nested = direct.clone();
        let viewport = Rect::from_xywh(30.0, 31.0, 80.0, 35.0).unwrap();
        let cell = Rect::from_xywh(33.0, 34.0, 27.0, 20.0).unwrap();
        Painter::new(&mut direct)
            .with_scale(scale)
            .clipped_scene(Some(cell), |painter| {
                painter.fill_rounded(cell, 0.0, Color::from_rgba8(120, 230, 180, 255));
            });
        Painter::new(&mut nested)
            .with_scale(scale)
            .clipped_scene(Some(viewport), |painter| {
                painter.clipped_scene(Some(cell), |painter| {
                    painter.fill_rounded(cell, 0.0, Color::from_rgba8(120, 230, 180, 255));
                });
            });
        assert_eq!(direct.data(), nested.data(), "scale {scale}");
        assert_ne!(
            direct.pixel((40.0 * scale) as u32, (40.0 * scale) as u32),
            Some(tiny_skia::PremultipliedColorU8::from_rgba(30, 34, 38, 255).unwrap())
        );
    }
}
