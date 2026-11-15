//! 【框形标注】【聚焦遮罩】周围统一变暗，内部保持原图；旋转和圆角沿用标注几何

use crate::{
    renderer::Painter,
    types::{Annotation, AnnotationShape},
};
use tiny_skia::{Color, FillRule, PathBuilder, Rect};

/// 【框形标注】【聚焦资格】按 annotation 返回矩形或椭圆是否启用聚焦，其他形状返回 false
pub fn enabled(annotation: &Annotation) -> bool {
    matches!(
        annotation.shape,
        AnnotationShape::Rectangle { focus: true, .. }
            | AnnotationShape::Circle { focus: true, .. }
    )
}

/// 【框形标注】【聚焦绘制】按 painter 和 annotation 绘制画布减去目标形状的遮罩，无返回值
pub fn draw(painter: &mut Painter<'_>, annotation: &Annotation) {
    if !enabled(annotation) {
        return;
    }
    let Some(bounds) = painter.drawing_bounds().or(annotation.frozen_bounds) else {
        return;
    };
    let (start, end, radius, ellipse) = match annotation.shape {
        AnnotationShape::Rectangle {
            start, end, radius, ..
        } => (start, end, annotation.style_units().canvas(radius), false),
        AnnotationShape::Circle { start, end, .. } => (start, end, 0.0, true),
        _ => return,
    };
    let Some(rect) = Rect::from_ltrb(
        start.0.min(end.0),
        start.1.min(end.1),
        start.0.max(end.0),
        start.1.max(end.1),
    ) else {
        return;
    };
    // 1. 【框形标注】【目标轮廓】只旋转透明孔洞，外部遮罩保持画布范围
    let path = if ellipse {
        PathBuilder::from_oval(rect)
    } else {
        crate::features::shape::path(rect, radius, 0.0, (0.0, 0.0))
    };
    let Some(path) = path.and_then(|path| {
        path.transform(crate::features::annotation_rotation::geometry::transform(
            annotation.rotation_center(),
            annotation.rotation_degrees,
        ))
    }) else {
        return;
    };
    let mut builder = PathBuilder::new();
    builder.push_rect(bounds);
    builder.push_path(&path);
    // 2. 【框形标注】【周围压暗】偶奇填充保留内部像素，透明遮罩支持预览缓存与最终导出
    if let Some(path) = builder.finish() {
        painter.clipped_scene(Some(bounds), |painter| {
            painter.fill_path_rule(&path, Color::from_rgba8(0, 0, 0, 150), FillRule::EvenOdd);
        });
    }
}

#[cfg(test)]
mod tests;
