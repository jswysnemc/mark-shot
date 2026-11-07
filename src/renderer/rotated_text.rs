//! 【文字旋转】【区域图像】保留原始字形相位，仅为可见文字区域分配临时缓冲，再执行一次角度变换

use super::Painter;
use crate::{features::annotation_rotation::geometry, types::Annotation};
use tiny_skia::{FilterQuality, IntRect, Pixmap, PixmapPaint, Rect, Transform};

/// 【文字旋转】【原图区域】按 painter 和 annotation 返回本次可见的未旋转文字范围，包含双线性采样留量
fn visible(painter: &mut Painter<'_>, annotation: &Annotation) -> Option<Rect> {
    let origin = painter.origin();
    let size = painter.size();
    let viewport = Rect::from_xywh(origin.0, origin.1, size.0, size.1)?;
    let inverse = geometry::bounds(
        viewport,
        annotation.rotation_center(),
        -annotation.rotation_degrees,
    )
    .outset(2.0 / painter.scale(), 2.0 / painter.scale())?;
    let body = annotation
        .text_painted_bounds()
        .outset(annotation.visual_pad(), annotation.visual_pad())?;
    body.intersect(&inverse)
}

/// 【文字旋转】【固定像素】按 area、scale 和 phase 返回原始画布物理网格中的整数范围，退化时返回空值
fn pixels(area: Rect, scale: f32, phase: (f64, f64)) -> Option<IntRect> {
    let at = |value: f32, phase: f64| f64::from(value) * f64::from(scale) - phase;
    IntRect::from_ltrb(
        at(area.left(), phase.0).floor() as i32,
        at(area.top(), phase.1).floor() as i32,
        at(area.right(), phase.0).ceil() as i32,
        at(area.bottom(), phase.1).ceil() as i32,
    )
}

/// 【文字旋转】【物理矩阵】按 painter、annotation、source 和 phase 返回文字像素到当前缓冲的矩阵，避免逻辑比例往返舍入
fn transform(
    painter: &Painter<'_>,
    annotation: &Annotation,
    source: IntRect,
    phase: (f64, f64),
) -> Transform {
    let scale = f64::from(painter.scale());
    let center = annotation.rotation_center();
    let center = (f64::from(center.0) * scale, f64::from(center.1) * scale);
    let source = (
        f64::from(source.x()) + phase.0,
        f64::from(source.y()) + phase.1,
    );
    let origin = painter.origin();
    let origin = (
        (f64::from(origin.0) * scale - phase.0).round() + phase.0,
        (f64::from(origin.1) * scale - phase.1).round() + phase.1,
    );
    let (sin, cos) = geometry::normalized(annotation.rotation_degrees)
        .to_radians()
        .sin_cos();
    Transform::from_row(
        cos as f32,
        sin as f32,
        -sin as f32,
        cos as f32,
        (center.0 + (source.0 - center.0) * cos - (source.1 - center.1) * sin - origin.0) as f32,
        (center.1 + (source.0 - center.0) * sin + (source.1 - center.1) * cos - origin.1) as f32,
    )
}

/// 【文字旋转】【正文合成】按 painter、annotation 和 body 绘制旋转文字，无返回值，只分配当前可见区域的字形图像
pub(super) fn draw(
    painter: &mut Painter<'_>,
    annotation: &Annotation,
    body: impl FnOnce(&mut Painter<'_>),
) {
    let Some(area) = visible(painter, annotation) else {
        return;
    };
    let phase = painter.pixel_phase();
    let Some(bounds) = pixels(area, painter.scale(), phase) else {
        return;
    };
    let Some(mut image) = Pixmap::new(bounds.width(), bounds.height()) else {
        return;
    };
    let origin = (
        ((f64::from(bounds.x()) + phase.0) / f64::from(painter.scale())) as f32,
        ((f64::from(bounds.y()) + phase.1) / f64::from(painter.scale())) as f32,
    );
    // 1. 【文字旋转】【原始字形】沿用未旋转坐标和实际字体缓存，不改变正文布局、字号或段落宽度
    body(&mut painter.sub(&mut image).with_origin(origin));
    let transform = transform(painter, annotation, bounds, phase);
    // 2. 【文字旋转】【单次合成】旋转后的文字和背景只合成一次，父缓冲自动裁切当前损坏范围
    painter.canvas().draw_pixmap(
        0,
        0,
        image.as_ref(),
        &PixmapPaint {
            quality: FilterQuality::Bilinear,
            ..PixmapPaint::default()
        },
        transform,
        None,
    );
}
