//! 【标注选框】【正文留白】类矩形选框避开描边，缩放仍以正文边界作为参照

use super::chrome;
use crate::{
    features::annotation_style::window::WindowUnits,
    types::{Annotation, AnnotationShape, SelectionHandle},
};
use tiny_skia::Rect;

/// 【标注选框】【留白范围】按annotation返回是否需要扩大选框，其他工具保留专用几何
pub(crate) fn padded(annotation: &Annotation) -> bool {
    matches!(
        annotation.shape,
        AnnotationShape::Rectangle { .. }
            | AnnotationShape::Circle { .. }
            | AnnotationShape::Marker { .. }
    )
}

/// 【标注选框】【可见边界】按annotation与window返回描边外保留六窗口像素的选框，不改变正文或旋转中心
pub(crate) fn bounds(annotation: &Annotation, window: WindowUnits) -> Option<Rect> {
    let body = chrome::bounds(annotation)?;
    if !padded(annotation) {
        return Some(body);
    }
    let padding = window.stroke(annotation.stroke_width, 1.5, None) * 0.5 + window.canvas(6.0);
    body.outset(padding, padding)
}

/// 【标注选框】【窗口重绘】按annotation与window返回类矩形正文及全部控制点的重绘范围，其他形状返回空值
pub(crate) fn editing_bounds(annotation: &Annotation, window: WindowUnits) -> Option<Rect> {
    if !padded(annotation) {
        return None;
    }
    let padding = window.canvas(8.0);
    let frame = bounds(annotation, window)?.outset(padding, padding)?;
    let mut area = annotation
        .rotated_bounds(frame)
        .join(&annotation.damage_bbox(false))?;
    if let Some((_, point)) = annotation.generic_rotation_handle_in_window(window) {
        let handle = Rect::from_xywh(
            point.0 - padding,
            point.1 - padding,
            padding * 2.0,
            padding * 2.0,
        )?;
        area = area.join(&handle)?;
    }
    Some(area)
}

/// 【标注选框】【缩放指针】按annotation、handle、start与current返回反旋转正文指针，保留按下位置相对正文边缘的偏移
pub(crate) fn resize_pointer(
    annotation: &Annotation,
    handle: SelectionHandle,
    start: (f64, f64),
    current: (f64, f64),
) -> chrome::Point {
    let mut current = annotation.to_unrotated((current.0 as f32, current.1 as f32));
    if !padded(annotation) {
        return current;
    }
    let Some(body) = chrome::bounds(annotation) else {
        return current;
    };
    let start = annotation.to_unrotated((start.0 as f32, start.1 as f32));
    use SelectionHandle::*;
    // 1. 【标注选框】【水平偏移】左右边与角点使用按下时的正文边缘，避免首次移动把留白写入尺寸
    if matches!(handle, Left | TopLeft | BottomLeft) {
        current.0 += body.left() - start.0;
    } else if matches!(handle, Right | TopRight | BottomRight) {
        current.0 += body.right() - start.0;
    }
    // 2. 【标注选框】【垂直偏移】上下边与角点保持相同规则，旋转后仍使用未旋转正文坐标
    if matches!(handle, Top | TopLeft | TopRight) {
        current.1 += body.top() - start.1;
    } else if matches!(handle, Bottom | BottomLeft | BottomRight) {
        current.1 += body.bottom() - start.1;
    }
    current
}
