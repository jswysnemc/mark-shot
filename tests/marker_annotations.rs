//! 【形状测试】【持久标注】验证位置、尺寸和输出绘制保留标记属性

use std::collections::HashMap;

use mark_shot::renderer::{Painter, draw_annotation};
use mark_shot::types::{
    Annotation, AnnotationShape, MarkerShape, SignedRect, StrokeGroup, ToolSettings,
};
use tiny_skia::{Color, Pixmap, Rect};

/// 【形状测试】【标注构造】使用 kind、filled 构造反向拖动标记；返回已计算边界的标注
fn annotation(kind: MarkerShape, filled: bool) -> Annotation {
    let color = Color::from_rgba8(255, 30, 20, 255);
    let mut annotation = Annotation {
        style_scale: 1.0,
        rotation_degrees: 0.0,
        frozen_bounds: None,
        id: 23,
        shape: AnnotationShape::Marker {
            start: (80.0, 40.0),
            end: (20.0, 100.0),
            kind,
            filled,
        },
        color,
        shadow_color: mark_shot::renderer::shadow_color_for(color),
        stroke_width: 3.0,
        bbox: Rect::from_xywh(20.0, 40.0, 60.0, 60.0).unwrap(),
    };
    annotation.update_bbox();
    annotation
}

/// 【形状测试】【比例移动】缩放与平移保留稳定形状、填充、笔宽和标识，同时同步逻辑边界
#[test]
fn scaling_and_translation_preserve_marker_properties() {
    let original = annotation(MarkerShape::Spade, false);
    let mut scaled = original.scaled(2.0);
    assert_eq!(scaled.id, original.id);
    assert_eq!(scaled.stroke_width, 6.0);
    assert_eq!(scaled.color, original.color);
    assert_eq!(scaled.bbox.left(), original.bbox.left() * 2.0);
    assert_eq!(scaled.bbox.top(), original.bbox.top() * 2.0);
    assert_eq!(scaled.bbox.width(), original.bbox.width() * 2.0);
    assert_eq!(scaled.bbox.height(), original.bbox.height() * 2.0);
    scaled.translate_mut(-30.0, 25.0);
    assert!(matches!(
        scaled.shape,
        AnnotationShape::Marker {
            start: (130.0, 105.0),
            end: (10.0, 225.0),
            kind: MarkerShape::Spade,
            filled: false
        }
    ));
    assert!(scaled.bbox.left() < 10.0 && scaled.bbox.right() > 130.0);
    assert!(scaled.bbox.top() < 105.0 && scaled.bbox.bottom() > 225.0);
    assert!(matches!(
        original.shape,
        AnnotationShape::Marker {
            start: (80.0, 40.0),
            end: (20.0, 100.0),
            ..
        }
    ));
}

/// 【形状测试】【反向调整】跨过相对边界调整尺寸时保留形状种类与填充，输出边界仍为规范矩形
#[test]
fn reversed_resize_preserves_kind_and_normalizes_bounds() {
    let original = annotation(MarkerShape::SpeechBubble, true);
    let resized = original.resize_to_bbox(SignedRect {
        left: 200.0,
        top: 180.0,
        right: 100.0,
        bottom: 100.0,
    });
    let AnnotationShape::Marker {
        start,
        end,
        kind,
        filled,
    } = resized.shape
    else {
        panic!("resizing changed the annotation type")
    };
    assert_eq!(kind, MarkerShape::SpeechBubble);
    assert!(filled);
    assert!(start.0 < end.0 && start.1 > end.1);
    assert!(start.0 > 100.0 && end.0 < 200.0);
    assert!(end.1 > 100.0 && start.1 < 180.0);
    assert!(resized.bbox.left() < start.0 && resized.bbox.right() > end.0);
    assert!(resized.bbox.top() < end.1 && resized.bbox.bottom() > start.1);
    assert_eq!(resized.stroke_width, original.stroke_width);
}

/// 【形状测试】【持久绘制】填充与空心使用同一持久出口，所有形状输出非透明像素
#[test]
fn persistent_drawing_covers_every_shape_and_keeps_hollow_interiors() {
    for kind in MarkerShape::ALL {
        let annotation = annotation(kind, true);
        let mut output = Pixmap::new(120, 140).unwrap();
        draw_annotation(
            &mut Painter::new(&mut output),
            &annotation,
            false,
            &mut HashMap::new(),
            &HashMap::new(),
            None,
        );
        assert!(
            output.pixels().iter().any(|pixel| pixel.alpha() > 0),
            "{}",
            kind.key()
        );
    }
    let mut output = Pixmap::new(120, 140).unwrap();
    let hollow = annotation(MarkerShape::Square, false);
    draw_annotation(
        &mut Painter::new(&mut output),
        &hollow,
        false,
        &mut HashMap::new(),
        &HashMap::new(),
        None,
    );
    assert_eq!(output.pixel(50, 70).unwrap().alpha(), 0);
    let mut filled = Pixmap::new(120, 140).unwrap();
    draw_annotation(
        &mut Painter::new(&mut filled),
        &annotation(MarkerShape::Square, true),
        false,
        &mut HashMap::new(),
        &HashMap::new(),
        None,
    );
    assert_eq!(filled.pixel(50, 70).unwrap().alpha(), 255);
}

/// 【形状测试】【独立笔宽】修改标记笔宽只影响标记分组，保持所有旧工具分组索引稳定
#[test]
fn marker_stroke_is_an_independent_appended_group() {
    let mut settings = ToolSettings::default();
    let prior: Vec<_> = StrokeGroup::ALL
        .into_iter()
        .filter(|group| *group != StrokeGroup::Marker)
        .map(|group| (group, settings.stroke(group)))
        .collect();
    settings.set_stroke(StrokeGroup::Marker, 17.0);
    for (group, width) in prior {
        assert_eq!(settings.stroke(group), width);
    }
    assert_eq!(StrokeGroup::Marker as usize, 7);
    assert_eq!(StrokeGroup::Laser as usize, 6);
    assert_eq!(
        StrokeGroup::of_shape(&annotation(MarkerShape::Triangle, true).shape),
        StrokeGroup::Marker
    );
}
