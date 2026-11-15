//! 【普通标注旋转验证】【跨模块】对照原版旋转几何，并验证实际正文和导出缩放

#[path = "annotation_rotation/geometry.rs"]
mod geometry;
#[path = "annotation_rotation/partial.rs"]
mod partial;
#[path = "annotation_rotation/rendering.rs"]
mod rendering;
#[path = "annotation_rotation/text_bounds.rs"]
mod text_bounds;

use mark_shot::types::{Annotation, AnnotationShape};
use tiny_skia::{Color, Rect};

/// 【普通标注旋转验证】【标注构造】按 shape 返回中心位于测试画布内的持久标注，原始角度为零
fn annotation(shape: AnnotationShape) -> Annotation {
    let mut annotation = Annotation {
        style_scale: 1.0,
        id: 1,
        rotation_degrees: 0.0,
        frozen_bounds: None,
        shape,
        color: Color::from_rgba8(51, 65, 85, 255),
        shadow_color: Color::TRANSPARENT,
        stroke_width: 8.0,
        bbox: Rect::from_xywh(1.0, 1.0, 1.0, 1.0).unwrap(),
    };
    annotation.update_bbox();
    annotation
}

/// 【普通标注旋转验证】【矩形构造】按 bounds 返回对应原版参考的矩形，选框不依赖描边留量
fn rectangle(bounds: Rect) -> Annotation {
    annotation(AnnotationShape::Rectangle {
        radius: 0.0,
        fixed_ratio: false,
        start: (bounds.left(), bounds.top()),
        end: (bounds.right(), bounds.bottom()),
        filled: false,
    })
}
