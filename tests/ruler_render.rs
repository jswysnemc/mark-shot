//! 【尺子测试】【绘制一致】验证持久正文、旋转刻度、局部绘制和原图说明

use std::collections::HashMap;

use mark_shot::features::ruler::labels::{InfoLabel, prepare_all};
use mark_shot::features::ruler::{Measurement, RulerGeometry, RulerMode};
use mark_shot::renderer::{Painter, draw_annotation, rebuild_annotations_layer, ruler_overlay};
use mark_shot::types::{Annotation, AnnotationShape};
use tiny_skia::{Color, IntRect, Pixmap, Rect};

/// 【尺子测试】【持久构造】按 mode 构造半透明尺子；length 支持 rotation，返回完整持久标注
fn annotation(mode: RulerMode, rotation: f32) -> Annotation {
    let mut measurement = Measurement::new(mode, (130.5, 140.5), 1.5).unwrap();
    measurement.geometry = match mode {
        RulerMode::Length => RulerGeometry::Length {
            start: (130.5, 140.5),
            end: (280.5, 240.5),
            rotation,
        },
        RulerMode::Angle => RulerGeometry::Angle {
            points: [(130.5, 140.5), (300.5, 140.5), (210.5, 260.5)],
            count: 3,
        },
    };
    let color = Color::from_rgba8(240, 55, 35, 180);
    let mut result = Annotation {
        id: 9,
        shape: AnnotationShape::Ruler { measurement },
        color,
        shadow_color: Color::TRANSPARENT,
        stroke_width: 3.0,
        bbox: measurement.bounds().unwrap(),
    };
    result.update_bbox();
    result
}

/// 【尺子测试】【正文绘制】绘制 ann 到 scale 对应的透明层；bounds 指定固定允许范围，返回像素图
fn render(ann: &Annotation, scale: f32, bounds: Rect) -> Pixmap {
    let mut pixels = Pixmap::new((640.0 * scale) as u32, (480.0 * scale) as u32).unwrap();
    let mut fonts = mark_shot::fonts::bundled();
    draw_annotation(
        &mut Painter::new(&mut pixels)
            .with_scale(scale)
            .with_drawing_bounds(Some(bounds))
            .with_text(&mut fonts, &mut cosmic_text::SwashCache::new()),
        ann,
        false,
        &mut HashMap::new(),
        &HashMap::new(),
        None,
    );
    pixels
}

/// 【尺子测试】【正文与单位】两类尺子经持久绘制入口生成像素，比例映射保留原图测量和样式
#[test]
fn both_ruler_bodies_render_through_the_persistent_annotation_entry() {
    let bounds = Rect::from_xywh(0.0, 0.0, 640.0, 480.0).unwrap();
    for mode in RulerMode::ALL {
        let ann = annotation(mode, 30.0);
        let output = render(&ann, 1.0, bounds);
        assert!(output.pixels().iter().any(|pixel| pixel.alpha() > 0));
        let scaled = ann.scaled(2.0);
        let AnnotationShape::Ruler {
            measurement: original,
        } = ann.shape
        else {
            panic!("ruler type changed")
        };
        let AnnotationShape::Ruler {
            measurement: transformed,
        } = scaled.shape
        else {
            panic!("ruler type changed")
        };
        assert_eq!(original.info_text(), transformed.info_text());
        assert_eq!(
            transformed.unit(scaled.stroke_width),
            original.unit(ann.stroke_width) * 2.0
        );
    }
}

/// 【尺子测试】【局部刻度】局部重建保留固定数字边界与半透明像素，不因临时缓冲尺寸改变数字排布
#[test]
fn partial_ruler_layer_rebuild_matches_a_full_layer_at_fractional_scale() {
    let bounds = Rect::from_xywh(0.0, 0.0, 640.0, 480.0).unwrap();
    for mode in RulerMode::ALL {
        for rotation in [0.0, 37.0] {
            let ann = annotation(mode, rotation);
            let original = render(&ann, 1.2, bounds);
            let mut layer = original.clone();
            let mut fonts = mark_shot::fonts::bundled();
            rebuild_annotations_layer(
                &mut Painter::new(&mut layer)
                    .with_scale(1.2)
                    .with_drawing_bounds(Some(bounds))
                    .with_text(&mut fonts, &mut cosmic_text::SwashCache::new()),
                &[ann],
                &mut HashMap::new(),
                &HashMap::new(),
                None,
                Some(Rect::from_xywh(96.0, 95.0, 180.0, 180.0).unwrap()),
            );
            let differences: Vec<_> = layer
                .pixels()
                .iter()
                .zip(original.pixels())
                .enumerate()
                .filter(|(_, (first, second))| first != second)
                .map(|(index, (first, second))| (index % 768, index / 768, *first, *second))
                .collect();
            assert!(
                differences.is_empty(),
                "mode={mode:?}, rotation={rotation}, mismatches={}, first={:?}",
                differences.len(),
                &differences[..differences.len().min(8)]
            );
        }
    }
}

/// 【尺子测试】【说明局部重绘】说明只在已恢复区域叠加，重复局部重绘不会加深半透明背景
#[test]
fn clipped_labels_restore_exactly_without_touching_other_pixels() {
    let mut fonts = mark_shot::fonts::bundled();
    let mut cache = cosmic_text::SwashCache::new();
    let bounds = Rect::from_xywh(0.0, 0.0, 640.0, 480.0).unwrap();
    let AnnotationShape::Ruler { measurement } = annotation(RulerMode::Length, 0.0).shape else {
        panic!("ruler type changed")
    };
    let labels = prepare_all(std::iter::once((measurement, 3.0)), bounds, &mut fonts);
    let mut base = Pixmap::new(768, 576).unwrap();
    base.fill(Color::from_rgba8(30, 34, 38, 255));
    let mut full = base.clone();
    ruler_overlay::draw_clipped(
        &mut full,
        &labels,
        None,
        (0.0, 0.0),
        1.2,
        None,
        &mut fonts,
        &mut cache,
    );
    let dirty = Rect::from_xywh(labels[0].rect.left(), labels[0].rect.top(), 70.0, 35.0).unwrap();
    let pixels = mark_shot::renderer::scale_rect(&dirty, 1.2)
        .unwrap()
        .round_out()
        .unwrap();
    let pixels =
        IntRect::from_xywh(pixels.x(), pixels.y(), pixels.width(), pixels.height()).unwrap();
    let blank = base.clone_rect(pixels).unwrap();
    let mut partial = full.clone();
    mark_shot::renderer::paste(&mut partial, &blank, pixels.x(), pixels.y());
    ruler_overlay::draw_clipped(
        &mut partial,
        &labels,
        None,
        (0.0, 0.0),
        1.2,
        Some(&dirty),
        &mut fonts,
        &mut cache,
    );
    assert!(
        partial.data() == full.data(),
        "first mismatch={:?}",
        partial
            .pixels()
            .iter()
            .zip(full.pixels())
            .position(|(first, second)| first != second)
    );
}

/// 【尺子测试】【真实分数相位】真实 Wayland 测量说明在非整数圆角相位下，整帧与移位损坏范围绘制逐像素一致
#[test]
fn fractional_live_label_corners_match_after_shifted_damage() {
    let mut measurement = Measurement::new(RulerMode::Length, (224.0, 220.0), 1.5957447).unwrap();
    measurement.geometry = RulerGeometry::Length {
        start: (224.0, 220.0),
        end: (384.0, 345.0),
        rotation: 0.0,
    };
    let mut fonts = mark_shot::fonts::bundled();
    let mut cache = cosmic_text::SwashCache::new();
    let bounds = Rect::from_xywh(30.0, 50.0, 1003.0, 565.0).unwrap();
    let labels = prepare_all(std::iter::once((measurement, 3.0)), bounds, &mut fonts);
    let mut base = Pixmap::new(1280, 800).unwrap();
    base.fill(Color::from_rgba8(30, 34, 38, 255));
    let mut full = base.clone();
    ruler_overlay::draw_clipped(
        &mut full,
        &labels,
        None,
        (0.0, 0.0),
        1.2,
        None,
        &mut fonts,
        &mut cache,
    );
    for (x, y) in [(0.0, 0.0), (3.0, 4.0), (-12.0, -15.0)] {
        let dirty = Rect::from_xywh(
            labels[0].rect.left() - 20.0 + x,
            labels[0].rect.top() - 20.0 + y,
            labels[0].rect.width() + 40.0,
            labels[0].rect.height() + 40.0,
        )
        .unwrap();
        let pixels = mark_shot::renderer::scale_rect(&dirty, 1.2)
            .unwrap()
            .round_out()
            .unwrap();
        let blank = base.clone_rect(pixels).unwrap();
        let mut partial = full.clone();
        mark_shot::renderer::paste(&mut partial, &blank, pixels.x(), pixels.y());
        ruler_overlay::draw_clipped(
            &mut partial,
            &labels,
            None,
            (0.0, 0.0),
            1.2,
            Some(&dirty),
            &mut fonts,
            &mut cache,
        );
        assert!(
            partial.data() == full.data(),
            "shift={x},{y}, first mismatch={:?}",
            partial
                .pixels()
                .iter()
                .zip(full.pixels())
                .position(|(first, second)| first != second)
        );
    }
}

/// 【尺子测试】【最后绘制】全部测量说明位于后来标注之上，导出中仍可读取测量信息
#[test]
fn measurement_labels_can_be_drawn_after_all_annotation_bodies() {
    let mut pixels = Pixmap::new(640, 480).unwrap();
    pixels.fill(Color::from_rgba8(0, 0, 255, 255));
    let label = InfoLabel {
        rect: Rect::from_xywh(100.0, 100.0, 360.0, 30.0).unwrap(),
        unit: 1.0,
        lines: vec!["225 x 150 px   diag 270 px   area 33750 px".to_string()],
        font_size: 13.0,
    };
    let mut fonts = mark_shot::fonts::bundled();
    ruler_overlay::draw_labels(
        &mut Painter::new(&mut pixels).with_text(&mut fonts, &mut cosmic_text::SwashCache::new()),
        &[label],
    );
    assert_ne!(
        pixels.pixel(110, 115).unwrap(),
        tiny_skia::PremultipliedColorU8::from_rgba(0, 0, 255, 255).unwrap()
    );
    assert!(
        pixels
            .pixels()
            .iter()
            .any(|pixel| pixel.red() > 180 && pixel.green() > 180)
    );
    assert_eq!(
        pixels.pixel(50, 50).unwrap(),
        tiny_skia::PremultipliedColorU8::from_rgba(0, 0, 255, 255).unwrap()
    );
}

/// 【尺子测试】【移动与调整】持久平移和范围缩放更新测量值，不改变工具种类和原图像素比例
#[test]
fn persistent_transforms_preserve_measurement_metadata() {
    let mut ann = annotation(RulerMode::Length, 0.0);
    let AnnotationShape::Ruler {
        measurement: original,
    } = ann.shape
    else {
        panic!("ruler type changed")
    };
    ann.translate_mut(-40.0, 35.0);
    let AnnotationShape::Ruler { measurement: moved } = ann.shape else {
        panic!("ruler type changed")
    };
    assert_eq!(moved.info_text(), original.info_text());
    assert_eq!(moved.start(), (90.5, 175.5));
    let resized = ann.resize_to_bbox(mark_shot::types::SignedRect {
        left: 90.5,
        top: 175.5,
        right: 390.5,
        bottom: 375.5,
    });
    let AnnotationShape::Ruler {
        measurement: resized,
    } = resized.shape
    else {
        panic!("ruler type changed")
    };
    assert_eq!(resized.pixel_scale, original.pixel_scale);
    assert_eq!(resized.length_metrics().unwrap().0, 450);
}
