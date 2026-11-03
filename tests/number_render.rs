//! 【序号测试】【像素绘制】覆盖七种标签、透明度、旋转、局部重绘和原始层叠顺序

use std::collections::HashMap;

use mark_shot::features::number::{NumberMark, NumberStyle};
use mark_shot::renderer::{Painter, annotation_stack, draw_annotation, rebuild_annotations_layer};
use mark_shot::types::{Annotation, AnnotationShape};
use tiny_skia::{Color, Pixmap, Rect};

/// 【序号测试】【标注构造】返回 style、number 对应的序号，bubble 和 tip 指定定点，rotation 使用弧度
fn annotation(
    style: NumberStyle,
    number: u32,
    bubble: (f32, f32),
    tip: (f32, f32),
    rotation: f32,
) -> Annotation {
    let mut mark = NumberMark::new(bubble, number, style).unwrap();
    mark.tip = tip;
    mark.rotation = rotation;
    let mut annotation = Annotation {
        id: 2,
        shape: AnnotationShape::Number { mark },
        color: Color::from_rgba8(210, 30, 50, 128),
        shadow_color: Color::TRANSPARENT,
        stroke_width: 3.0,
        bbox: Rect::from_xywh(0.0, 0.0, 1.0, 1.0).unwrap(),
    };
    annotation.update_bbox();
    annotation
}

/// 【序号测试】【透明绘制】按 size、scale、origin 绘制 annotations 到透明画布；with_fonts 决定是否绘制标签，返回像素图
fn render(
    annotations: &[Annotation],
    size: (u32, u32),
    scale: f32,
    origin: (f32, f32),
    with_fonts: bool,
) -> Pixmap {
    let mut image = Pixmap::new(size.0, size.1).unwrap();
    let mut fonts = mark_shot::fonts::bundled();
    let mut cache = cosmic_text::SwashCache::new();
    let mut painter = Painter::new(&mut image)
        .with_scale(scale)
        .with_origin(origin);
    if with_fonts {
        painter = painter.with_text(&mut fonts, &mut cache);
    }
    for annotation in annotations {
        draw_annotation(
            &mut painter,
            annotation,
            false,
            &mut HashMap::new(),
            &HashMap::new(),
            None,
        );
    }
    image
}

/// 【序号测试】【像素读取】返回 image 在 x、y 的预乘 RGBA，不改变颜色格式
fn pixel(image: &Pixmap, x: u32, y: u32) -> [u8; 4] {
    let start = (y as usize * image.width() as usize + x as usize) * 4;
    image.data()[start..start + 4].try_into().unwrap()
}

/// 【序号测试】【精确比较】验证 actual 与 expected 逐像素一致，只报告首个不同字节，不打印图像数据
fn same(actual: &Pixmap, expected: &Pixmap) {
    assert_eq!(
        (actual.width(), actual.height()),
        (expected.width(), expected.height())
    );
    assert_eq!(
        actual
            .data()
            .iter()
            .zip(expected.data())
            .position(|(a, b)| a != b),
        None
    );
}

/// 【序号测试】【七种标签】所有样式使用内置字形绘制标签，中文和天干无需系统字体
#[test]
fn every_style_renders_real_label_pixels_with_only_bundled_fonts() {
    for style in NumberStyle::ALL {
        let annotation = annotation(style, 27, (140.0, 140.0), (140.0, 140.0), 0.0);
        let with_text = render(
            std::slice::from_ref(&annotation),
            (320, 280),
            1.0,
            (0.0, 0.0),
            true,
        );
        let without_text = render(&[annotation], (320, 280), 1.0, (0.0, 0.0), false);
        assert!(
            with_text.data() != without_text.data(),
            "{style:?} has no glyph pixels"
        );
    }
    let first = annotation(
        NumberStyle::HeavenlyStem,
        1,
        (140.0, 140.0),
        (140.0, 140.0),
        0.0,
    );
    let second = annotation(
        NumberStyle::HeavenlyStem,
        2,
        (140.0, 140.0),
        (140.0, 140.0),
        0.0,
    );
    let first = render(&[first], (320, 280), 1.0, (0.0, 0.0), true);
    let second = render(&[second], (320, 280), 1.0, (0.0, 0.0), true);
    assert!(
        first.data() != second.data(),
        "different stems must not be missing-glyph boxes"
    );
}

/// 【序号测试】【独立透明度】半透明填充保留原始 alpha，白色标签和圆框边线遵循原版独立不透明规则
#[test]
fn translucent_bubbles_keep_an_opaque_white_ring_and_label() {
    let annotation = annotation(NumberStyle::Arabic, 1, (140.0, 140.0), (140.0, 140.0), 0.0);
    let image = render(&[annotation], (320, 280), 1.0, (0.0, 0.0), true);
    assert_eq!(pixel(&image, 130, 140), [105, 15, 25, 128]);
    assert!(pixel(&image, 156, 140)[3] > 128);
    assert!((132..149).any(|x| (131..149).any(|y| pixel(&image, x, y) == [255; 4])));
    assert_eq!(pixel(&image, 0, 0), [0; 4]);
}

/// 【序号测试】【原图导出】固定留量、笔宽和半透明尾部在导出缩放后与预览保持逐像素一致
#[test]
fn native_export_preserves_body_geometry_and_alpha() {
    let annotation = annotation(
        NumberStyle::Chinese,
        1001,
        (140.5, 140.5),
        (390.5, 200.5),
        0.7,
    );
    let preview = render(
        std::slice::from_ref(&annotation),
        (1280, 960),
        2.0,
        (0.0, 0.0),
        false,
    );
    let exported = render(
        &[annotation.scaled(2.0)],
        (1280, 960),
        1.0,
        (0.0, 0.0),
        false,
    );
    same(&preview, &exported);
}

/// 【序号测试】【局部一致】所有样式在旋转和分数缩放下局部重建，保持字形、白边和半透明尾部精确一致
#[test]
fn partial_rebuild_matches_full_render_for_every_style_and_rotation() {
    for style in NumberStyle::ALL {
        for rotation in [0.0, 0.65] {
            let annotation = annotation(style, 27, (140.5, 140.5), (390.5, 180.5), rotation);
            let expected = render(
                std::slice::from_ref(&annotation),
                (768, 576),
                1.2,
                (0.0, 0.0),
                true,
            );
            let mut actual = expected.clone();
            let mut fonts = mark_shot::fonts::bundled();
            rebuild_annotations_layer(
                &mut Painter::new(&mut actual)
                    .with_scale(1.2)
                    .with_text(&mut fonts, &mut cosmic_text::SwashCache::new()),
                &[annotation],
                &mut HashMap::new(),
                &HashMap::new(),
                None,
                Some(Rect::from_xywh(118.75, 112.5, 380.5, 230.25).unwrap()),
            );
            same(&actual, &expected);
        }
    }
}

/// 【序号测试】【多块尾部】长尾部跨越多个固定像素块，局部重建不改变三角边缘的透明度或采样相位
#[test]
fn leaders_larger_than_one_tile_keep_identical_partial_pixels() {
    let annotation = annotation(
        NumberStyle::UpperRoman,
        49,
        (130.7, 300.2),
        (1430.3, 480.4),
        0.18,
    );
    let expected = render(
        std::slice::from_ref(&annotation),
        (1920, 1080),
        1.2,
        (0.0, 0.0),
        true,
    );
    let mut actual = expected.clone();
    let mut fonts = mark_shot::fonts::bundled();
    for rect in [
        Rect::from_xywh(80.25, 260.75, 830.5, 390.25).unwrap(),
        Rect::from_xywh(850.75, 350.5, 720.25, 420.5).unwrap(),
    ] {
        rebuild_annotations_layer(
            &mut Painter::new(&mut actual)
                .with_scale(1.2)
                .with_text(&mut fonts, &mut cosmic_text::SwashCache::new()),
            std::slice::from_ref(&annotation),
            &mut HashMap::new(),
            &HashMap::new(),
            None,
            Some(rect),
        );
        same(&actual, &expected);
    }
}

/// 【序号测试】【拖动层序】已有序号暂存时保持较新半透明标注的原始层级，透明颜色只合成一次
#[test]
fn dragged_numbers_stay_below_newer_translucent_annotations() {
    let old = rectangle(
        1,
        Rect::from_xywh(80.0, 100.0, 160.0, 120.0).unwrap(),
        Color::from_rgba8(20, 50, 230, 100),
    );
    let pending = annotation(
        NumberStyle::Chinese,
        1001,
        (140.5, 140.5),
        (390.5, 200.5),
        0.2,
    );
    let newer = rectangle(
        3,
        Rect::from_xywh(100.0, 130.0, 260.0, 90.0).unwrap(),
        Color::from_rgba8(20, 230, 50, 130),
    );
    let expected = render(
        &[old.clone(), pending.clone(), newer.clone()],
        (768, 576),
        1.2,
        (0.0, 0.0),
        true,
    );
    let mut actual = Pixmap::new(768, 576).unwrap();
    let mut fonts = mark_shot::fonts::bundled();
    annotation_stack::draw_dragged(
        &mut Painter::new(&mut actual)
            .with_scale(1.2)
            .with_text(&mut fonts, &mut cosmic_text::SwashCache::new()),
        &[old, newer],
        &pending,
        1,
        &mut HashMap::new(),
        &HashMap::new(),
        None,
        None,
    );
    same(&actual, &expected);
}

/// 【序号测试】【背景标注】返回 id、rect、color 对应的填充矩形，用于层序回归
fn rectangle(id: u64, rect: Rect, color: Color) -> Annotation {
    let mut annotation = Annotation {
        id,
        shape: AnnotationShape::Rectangle {
            start: (rect.left(), rect.top()),
            end: (rect.right(), rect.bottom()),
            filled: true,
        },
        color,
        shadow_color: Color::TRANSPARENT,
        stroke_width: 1.0,
        bbox: rect,
    };
    annotation.update_bbox();
    annotation
}

/// 【序号测试】【标签约束】较长罗马数字和整数上限标签拟合在圆框内，不越界绘制白色字形
#[test]
fn long_labels_fit_inside_the_bubble_without_large_text_buffers() {
    for (style, number) in [
        (NumberStyle::UpperRoman, 10_888),
        (NumberStyle::Arabic, u32::MAX),
        (NumberStyle::Chinese, 1001),
    ] {
        let annotation = annotation(style, number, (140.0, 140.0), (140.0, 140.0), 0.0);
        let expected = render(
            std::slice::from_ref(&annotation),
            (320, 280),
            1.0,
            (0.0, 0.0),
            false,
        );
        let actual = render(&[annotation], (320, 280), 1.0, (0.0, 0.0), true);
        for y in 0..280 {
            for x in 0..320 {
                if (x as f32 - 140.0).hypot(y as f32 - 140.0) > 16.0 {
                    assert_eq!(
                        pixel(&actual, x, y),
                        pixel(&expected, x, y),
                        "glyph outside bubble at {x}, {y}"
                    );
                }
            }
        }
    }
}

/// 【序号测试】【字形分块】大比例导出的字形跨越一千零二十二像素，分段画布与完整局部画布仍保持精确一致
#[test]
fn oversized_rotated_glyphs_have_seamless_chunked_rendering() {
    let mut annotation = annotation(
        NumberStyle::UpperRoman,
        1000,
        (50.0, 50.0),
        (50.0, 50.0),
        0.35,
    )
    .scaled(40.0);
    annotation.stroke_width = 72.0 * 40.0;
    annotation.update_bbox();
    let origin = (1750.0, 1100.0);
    let expected = render(
        std::slice::from_ref(&annotation),
        (512, 512),
        1.0,
        origin,
        true,
    );
    let mut actual = Pixmap::new(512, 512).unwrap();
    for (x, y) in [(0, 0), (256, 0), (0, 256), (256, 256)] {
        let piece = render(
            std::slice::from_ref(&annotation),
            (256, 256),
            1.0,
            (origin.0 + x as f32, origin.1 + y as f32),
            true,
        );
        mark_shot::renderer::paste(&mut actual, &piece, x, y);
    }
    same(&actual, &expected);
    assert!(expected.pixels().iter().any(|pixel| pixel.red() == 255));
}
