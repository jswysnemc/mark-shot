//! 【文字存储】【矩形变换】按可见框比例映射原始矩形，保留隐藏宽高与首个定点

use crate::types::{Annotation, AnnotationShape, SelectionHandle};
use tiny_skia::Rect;

/// 【文字存储】【角点识别】按handle返回是否属于改变字号的四个角点，不包含边中点
pub(crate) fn corner(handle: SelectionHandle) -> bool {
    matches!(
        handle,
        SelectionHandle::TopLeft
            | SelectionHandle::TopRight
            | SelectionHandle::BottomLeft
            | SelectionHandle::BottomRight
    )
}

/// 【文字存储】【像素限制】按point、bounds和pixel返回原图范围内的定点，右下边界为最后源图像素
fn clamped(point: (f32, f32), bounds: Rect, pixel: f32) -> (f32, f32) {
    (
        point
            .0
            .clamp(bounds.left(), (bounds.right() - pixel).max(bounds.left())),
        point
            .1
            .clamp(bounds.top(), (bounds.bottom() - pixel).max(bounds.top())),
    )
}

/// 【文字存储】【连续映射】按annotation、old、target、handle、bounds和pixel映射存储矩形，无返回值，不把目标选框当作存储尺寸
pub(crate) fn resize(
    annotation: &mut Annotation,
    old: Rect,
    target: Rect,
    handle: SelectionHandle,
    bounds: Rect,
    pixel: f32,
) {
    let AnnotationShape::Text {
        start,
        frame,
        font_size,
        ..
    } = &mut annotation.shape
    else {
        return;
    };
    let width = frame.width.unwrap_or(annotation.bbox.width());
    let height = frame.height.unwrap_or(annotation.bbox.height());
    let ratio = (
        f64::from(target.width()) / f64::from(old.width()),
        f64::from(target.height()) / f64::from(old.height()),
    );
    let map = |point: (f32, f32)| {
        clamped(
            (
                (f64::from(target.left()) + (f64::from(point.0) - f64::from(old.left())) * ratio.0)
                    as f32,
                (f64::from(target.top()) + (f64::from(point.1) - f64::from(old.top())) * ratio.1)
                    as f32,
            ),
            bounds,
            pixel,
        )
    };
    // 1. 【文字存储】【原始两角】所有手势映射已有矩形，边中点不改首个定点
    let first = map(*start);
    let last = map((start.0 + width, start.1 + height));
    frame.anchor = Some(frame.anchor.unwrap_or(*start));
    *start = (first.0.min(last.0), first.1.min(last.1));
    frame.width = Some((first.0 - last.0).abs());
    frame.height = Some((first.1 - last.1).abs());
    frame.painted = None;
    // 2. 【文字存储】【角点字号】独立轴比例取最大值，保留原版二十至一千零十九点范围
    if corner(handle) {
        *font_size = (f64::from(*font_size) * ratio.0.max(ratio.1)).clamp(20.0, 1019.0) as f32;
    }
}

/// 【文字存储】【移动定点】按annotation、bounds和pixel限制移动后的首个定点，矩形位置和大小保持平移结果，无返回值
pub(crate) fn clamp_anchor(annotation: &mut Annotation, bounds: Rect, pixel: f32) {
    if let AnnotationShape::Text { start, frame, .. } = &mut annotation.shape {
        frame.anchor = Some(clamped(frame.anchor.unwrap_or(*start), bounds, pixel));
    }
}
