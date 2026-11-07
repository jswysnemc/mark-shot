//! 【普通标注旋转验证】【局部绘制】真实字形及矢量在窄缓冲内保持完整画布的像素相位

use super::*;
use cosmic_text::SwashCache;
use mark_shot::renderer::{Painter, draw_annotation};
use std::collections::HashMap;
use tiny_skia::Pixmap;

/// 【普通标注旋转验证】【指定区域】按 annotation、scale、origin 与 partial 返回实际正文画布，局部模式只绘制固定内部区域
fn rendered(annotation: &Annotation, scale: f32, origin: (f32, f32), partial: bool) -> Pixmap {
    let mut fonts = mark_shot::fonts::bundled();
    let mut cache = SwashCache::new();
    let mut editors = HashMap::new();
    let mut canvas = Pixmap::new((256.0 * scale) as u32, (256.0 * scale) as u32).unwrap();
    Painter::new(&mut canvas)
        .with_scale(scale)
        .with_origin(origin)
        .with_text(&mut fonts, &mut cache)
        .clipped(
            partial.then(|| Rect::from_xywh(64.0, 48.0, 128.0, 128.0).unwrap()),
            |painter| {
                draw_annotation(
                    painter,
                    annotation,
                    false,
                    &mut editors,
                    &HashMap::new(),
                    None,
                );
            },
        );
    canvas
}

/// 【普通标注旋转验证】【像素一致】按 annotation 核对三种角度、四种比例及两种原点，局部范围逐像素等于完整输出，范围外保持透明
fn assert_partial(mut annotation: Annotation) {
    for angle in [90.0, 45.0, -37.0] {
        annotation.rotation_degrees = angle;
        for scale in [1.0, 1.25, 1.5, 2.0] {
            for origin in [(0.0, 0.0), (11.2, -7.6)] {
                let full = rendered(&annotation, scale, origin, false);
                let partial = rendered(&annotation, scale, origin, true);
                let mut differences = 0;
                let mut covered = 0;
                for y in 0..partial.height() {
                    for x in 0..partial.width() {
                        let inside = x >= (64.0 * scale) as u32
                            && x < (192.0 * scale) as u32
                            && y >= (48.0 * scale) as u32
                            && y < (176.0 * scale) as u32;
                        let actual = partial.pixel(x, y).unwrap();
                        if inside {
                            let expected = full.pixel(x, y).unwrap();
                            differences += usize::from(actual != expected);
                            covered += usize::from(expected.alpha() > 0);
                        } else {
                            assert_eq!(actual.alpha(), 0, "outside ({x},{y})");
                        }
                    }
                }
                assert!(covered > 30, "rotation sample contained no body pixels");
                assert_eq!(
                    differences, 0,
                    "angle={angle} scale={scale} origin={origin:?} shape={:?}",
                    annotation.shape
                );
            }
        }
    }
}

/// 【普通标注旋转验证】【五类矢量】局部绘制与整图绘制逐像素一致，不重复叠加半透明荧光笔，无参数、无返回值
#[test]
fn rotated_vectors_keep_full_canvas_pixels_in_partial_buffers() {
    for annotation in super::rendering::vectors() {
        assert_partial(annotation);
    }
}

/// 【普通标注旋转验证】【实际字形】局部文字图像保留原始字形相位及双线性边缘，非整数原点也保持一致，无参数、无返回值
#[test]
fn rotated_text_keeps_full_canvas_pixels_in_partial_buffers() {
    assert_partial(super::rendering::text());
}

/// 【普通标注旋转验证】【预览缓存独立】缩放后的旧字形缓存不能裁切新比例正文，三种预览比例仍严格保持局部像素一致，无参数、无返回值
#[test]
fn scaled_text_with_old_painted_cache_keeps_exact_partial_pixels() {
    for scale in [0.8, 1.1, 1.4] {
        assert_partial(super::rendering::text().scaled(scale));
    }
}
