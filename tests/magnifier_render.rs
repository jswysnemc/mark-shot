//! 【放大镜测试】【像素一致性】验证冻结原图、透明度、圆形裁切、分块与标注层叠顺序

use std::collections::HashMap;

use mark_shot::features::magnifier::{Magnifier, MagnifierShape};
use mark_shot::renderer::{FrozenImage, Painter, draw_annotation, rebuild_annotations_layer};
use mark_shot::types::{Annotation, AnnotationShape};
use tiny_skia::{Color, Pixmap, Rect};

/// 【绘制测试】【原图构造】返回 width、height 像素的图像；pattern 为真时填充空间变化的预乘颜色，否则使用纯蓝色
fn image(width: u32, height: u32, pattern: bool) -> Pixmap {
    let mut pixels = Pixmap::new(width, height).unwrap();
    for (index, pixel) in pixels
        .data_mut()
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .enumerate()
    {
        if pattern {
            let x = index % width as usize;
            let y = index / width as usize;
            pixel.copy_from_slice(&[(x % 160) as u8, (y % 120) as u8, 100, 180]);
        } else {
            pixel.copy_from_slice(&[0, 0, 200, 255]);
        }
    }
    pixels
}

/// 【绘制测试】【图像标注】返回 rect、color 对应的填充矩形，id 用于区别历史对象
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
        stroke_width: 3.0,
        bbox: rect,
    };
    annotation.update_bbox();
    annotation
}

/// 【绘制测试】【双框标注】返回 shape、source、center、bounds 指定的放大镜；color 控制边框，不控制原图透明度
fn magnifier(
    shape: MagnifierShape,
    source: (f32, f32),
    center: (f32, f32),
    bounds: Rect,
    color: Color,
) -> Annotation {
    let magnifier = Magnifier::new(shape, source, 2.75, bounds, 1.0)
        .unwrap()
        .draft_to(center);
    let mut annotation = Annotation {
        id: 2,
        shape: AnnotationShape::Magnifier { magnifier },
        color,
        shadow_color: Color::TRANSPARENT,
        stroke_width: 3.0,
        bbox: bounds,
    };
    annotation.update_bbox();
    annotation
}

/// 【绘制测试】【正文合成】以 original 为冻结原图将 annotations 合成到透明层；scale 指定像素比例，返回完整像素图
fn render(original: &Pixmap, annotations: &[Annotation], scale: f32) -> Pixmap {
    let mut layer = Pixmap::new(
        (original.width() as f32 * scale).round() as u32,
        (original.height() as f32 * scale).round() as u32,
    )
    .unwrap();
    let bounds =
        Rect::from_xywh(0.0, 0.0, original.width() as f32, original.height() as f32).unwrap();
    let mut painter = Painter::new(&mut layer)
        .with_scale(scale)
        .with_frozen_image(FrozenImage::image(original, bounds));
    let mut editors = HashMap::new();
    let blurs = HashMap::new();
    for annotation in annotations {
        draw_annotation(&mut painter, annotation, false, &mut editors, &blurs, None);
    }
    layer
}

/// 【绘制测试】【像素读取】返回 image 在 x、y 的预乘 RGBA，不进行颜色格式转换
fn pixel(image: &Pixmap, x: u32, y: u32) -> [u8; 4] {
    let index = (y as usize * image.width() as usize + x as usize) * 4;
    image.data()[index..index + 4].try_into().unwrap()
}

/// 【绘制测试】【像素比较】验证 actual 与 expected 完全一致；失败时只显示首个差异索引，避免输出整幅图像
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

/// 【绘制测试】【冻结来源】源框区域先绘制红色标注，镜框仍显示冻结蓝色原图，不复制旧标注
#[test]
fn the_lens_samples_the_frozen_image_instead_of_earlier_annotations() {
    let original = image(640, 480, false);
    let old = rectangle(
        1,
        Rect::from_xywh(70.0, 100.0, 60.0, 60.0).unwrap(),
        Color::from_rgba8(255, 0, 0, 255),
    );
    let magnifier = magnifier(
        MagnifierShape::Rectangle,
        (100.0, 130.0),
        (320.0, 240.0),
        Rect::from_xywh(0.0, 0.0, 640.0, 480.0).unwrap(),
        Color::from_rgba8(255, 255, 255, 255),
    );
    let rendered = render(&original, &[old, magnifier], 1.0);
    assert_eq!(pixel(&rendered, 100, 130), [255, 0, 0, 255]);
    assert_eq!(pixel(&rendered, 320, 240), [0, 0, 200, 255]);
}

/// 【绘制测试】【独立透明度】边框透明不会隐藏镜框图像，半透明原图保持预乘颜色与原始透明度
#[test]
fn transparent_borders_do_not_hide_the_lens_or_whiten_transparent_pixels() {
    let mut original = image(640, 480, false);
    original.fill(Color::from_rgba8(0, 0, 200, 128));
    let magnifier = magnifier(
        MagnifierShape::Circle,
        (100.0, 130.0),
        (320.0, 240.0),
        Rect::from_xywh(0.0, 0.0, 640.0, 480.0).unwrap(),
        Color::TRANSPARENT,
    );
    let rendered = render(&original, &[magnifier], 1.0);
    assert_eq!(pixel(&rendered, 320, 240), [0, 0, 100, 128]);
    assert_eq!(pixel(&rendered, 0, 0), [0, 0, 0, 0]);
}

/// 【绘制测试】【圆形裁切】圆形镜框角落不绘制原图，圆心仍显示完整取样颜色
#[test]
fn circular_lenses_clip_the_rectangular_sample() {
    let original = image(640, 480, false);
    let annotation = magnifier(
        MagnifierShape::Circle,
        (100.0, 130.0),
        (320.0, 240.0),
        Rect::from_xywh(0.0, 0.0, 640.0, 480.0).unwrap(),
        Color::TRANSPARENT,
    );
    let AnnotationShape::Magnifier { magnifier } = annotation.shape else {
        unreachable!()
    };
    let rect = magnifier.lens_rect().unwrap();
    let rendered = render(&original, &[annotation], 1.0);
    assert_eq!(
        pixel(
            &rendered,
            rect.left().ceil() as u32 + 1,
            rect.top().ceil() as u32 + 1
        ),
        [0, 0, 0, 0]
    );
    assert_eq!(pixel(&rendered, 320, 240), [0, 0, 200, 255]);
}

/// 【绘制测试】【原图导出】绘图像素比例与标注导出比例对应时，镜框中心读取相同原图像素
#[test]
fn exported_geometry_reads_the_same_frozen_source_at_native_resolution() {
    let original = image(640, 480, true);
    let annotation = magnifier(
        MagnifierShape::Rectangle,
        (100.0, 130.0),
        (320.0, 240.0),
        Rect::from_xywh(0.0, 0.0, 640.0, 480.0).unwrap(),
        Color::TRANSPARENT,
    );
    let preview = render(&original, std::slice::from_ref(&annotation), 1.6);
    let mut exported = Pixmap::new(preview.width(), preview.height()).unwrap();
    let bounds = Rect::from_xywh(0.0, 0.0, 640.0, 480.0).unwrap();
    draw_annotation(
        &mut Painter::new(&mut exported)
            .with_frozen_image(FrozenImage::image(&original, bounds).mapped(1.6, (0.0, 0.0))),
        &annotation.scaled(1.6),
        false,
        &mut HashMap::new(),
        &HashMap::new(),
        None,
    );
    assert_eq!(pixel(&preview, 512, 384), pixel(&exported, 512, 384));
}

/// 【绘制测试】【固定分块】超过一个像素块的旋转镜框在分数缩放和局部重绘后保持逐像素一致
#[test]
fn large_rotated_lenses_have_identical_full_and_partial_rasterization() {
    let original = image(1600, 900, true);
    let mut annotation = magnifier(
        MagnifierShape::Rectangle,
        (220.4, 300.2),
        (850.3, 460.7),
        Rect::from_xywh(0.0, 0.0, 1600.0, 900.0).unwrap(),
        Color::from_rgba8(80, 210, 170, 150),
    );
    if let AnnotationShape::Magnifier { magnifier } = &mut annotation.shape {
        magnifier.size = (1250.7, 380.9);
        *magnifier = magnifier.constrained();
        magnifier.rotation = 34.0_f32.to_radians();
    }
    annotation.update_bbox();
    let annotations = [annotation];
    let expected = render(&original, &annotations, 1.2);
    let mut actual = expected.clone();
    let bounds = Rect::from_xywh(0.0, 0.0, 1600.0, 900.0).unwrap();
    for dirty in [
        Rect::from_xywh(31.25, 145.5, 920.75, 600.25).unwrap(),
        Rect::from_xywh(780.75, 60.5, 340.5, 750.5).unwrap(),
    ] {
        rebuild_annotations_layer(
            &mut Painter::new(&mut actual)
                .with_scale(1.2)
                .with_frozen_image(FrozenImage::image(&original, bounds)),
            &annotations,
            &mut HashMap::new(),
            &HashMap::new(),
            None,
            Some(dirty),
        );
        same(&actual, &expected);
    }
}

/// 【绘制测试】【拖动层序】拖动镜框不覆盖较新的半透明标注，也不重复合成原有图层
#[test]
fn dragging_keeps_newer_annotations_above_the_lens_exactly_once() {
    let original = image(640, 480, true);
    let older = rectangle(
        1,
        Rect::from_xywh(40.0, 100.0, 140.0, 120.0).unwrap(),
        Color::from_rgba8(230, 10, 20, 100),
    );
    let pending = magnifier(
        MagnifierShape::Rectangle,
        (100.0, 130.0),
        (320.0, 240.0),
        Rect::from_xywh(0.0, 0.0, 640.0, 480.0).unwrap(),
        Color::from_rgba8(255, 255, 255, 120),
    );
    let newer = rectangle(
        3,
        Rect::from_xywh(260.0, 210.0, 180.0, 80.0).unwrap(),
        Color::from_rgba8(20, 230, 10, 120),
    );
    let expected = render(
        &original,
        &[older.clone(), pending.clone(), newer.clone()],
        1.2,
    );
    let mut actual = Pixmap::new(expected.width(), expected.height()).unwrap();
    let bounds = Rect::from_xywh(0.0, 0.0, 640.0, 480.0).unwrap();
    mark_shot::renderer::annotation_stack::draw_dragged(
        &mut Painter::new(&mut actual)
            .with_scale(1.2)
            .with_frozen_image(FrozenImage::image(&original, bounds)),
        &[older, newer],
        &pending,
        1,
        &mut HashMap::new(),
        &HashMap::new(),
        None,
        None,
    );
    same(&actual, &expected);
}
