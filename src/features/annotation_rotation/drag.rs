//! 【普通标注】【手势计算】每次从原始快照计算角度或坐标，不累计旋转和缩放误差

use super::{geometry, resize};
use crate::{
    features::annotation_selection::chrome,
    types::{Annotation, AnnotationShape, SelectionHandle, SignedRect},
};
use tiny_skia::Rect;

#[derive(Clone, Copy)]
pub(crate) struct Options {
    pub bounds: Rect,
    pub pixel_scale: f32,
    pub keep_aspect: bool,
    pub snap_rotation: bool,
}

pub(crate) struct Change {
    pub annotation: Annotation,
    pub resized: Option<Rect>,
}

/// 【普通标注】【原图定点】按 annotation、bounds 与 pixel 限制变换后的定点到最后源图像素，矩形移动保持原版矩形平移语义，无返回值
fn clamp_points(annotation: &mut Annotation, bounds: Rect, pixel: f32, moving: bool) {
    let clamp = |point: &mut geometry::Point| {
        point.0 = point
            .0
            .clamp(bounds.left(), (bounds.right() - pixel).max(bounds.left()));
        point.1 = point
            .1
            .clamp(bounds.top(), (bounds.bottom() - pixel).max(bounds.top()));
    };
    match &mut annotation.shape {
        AnnotationShape::Line { line, .. } | AnnotationShape::Arrow { line, .. } => {
            *line = line.mapped(|mut point| {
                clamp(&mut point);
                point
            });
        }
        AnnotationShape::Pen { points } | AnnotationShape::Highlighter { points, .. } => {
            for point in points {
                clamp(point);
            }
        }
        AnnotationShape::Rectangle { start, end, .. }
        | AnnotationShape::Circle { start, end, .. }
        | AnnotationShape::Marker { start, end, .. }
            if !moving =>
        {
            clamp(start);
            clamp(end);
        }
        _ => return,
    }
    annotation.update_bbox();
}

/// 【普通标注】【位置约束】按 before、start、current 和 bounds 返回保持可见选择边界的平移量
fn translation(
    before: &Annotation,
    start: geometry::Point,
    current: geometry::Point,
    bounds: Rect,
) -> geometry::Point {
    let Some(frame) = before
        .rotated_bounds(chrome::bounds(before).unwrap_or(before.bbox))
        .intersect(&bounds)
    else {
        return (0.0, 0.0);
    };
    let x = (bounds.left() - frame.left(), bounds.right() - frame.right());
    let y = (bounds.top() - frame.top(), bounds.bottom() - frame.bottom());
    (
        (current.0 - start.0).clamp(x.0.min(x.1), x.0.max(x.1)),
        (current.1 - start.1).clamp(y.0.min(y.1), y.0.max(y.1)),
    )
}

/// 【普通标注】【原始手势】按 before、handle、start、current 和 options 返回新标注及可选缩放框，无可靠输入时返回空值
pub(crate) fn dragged(
    before: &Annotation,
    handle: SelectionHandle,
    start: (f64, f64),
    current: (f64, f64),
    mut options: Options,
) -> Option<Change> {
    options.keep_aspect |= matches!(
        before.shape,
        AnnotationShape::Rectangle {
            fixed_ratio: true,
            ..
        } | AnnotationShape::Circle {
            fixed_ratio: true,
            ..
        }
    );
    if !before.uses_generic_rotation() || !current.0.is_finite() || !current.1.is_finite() {
        return None;
    }
    let pixel = 1.0 / options.pixel_scale.max(0.001);
    let point = (
        current.0.clamp(
            f64::from(options.bounds.left()),
            f64::from((options.bounds.right() - pixel).max(options.bounds.left())),
        ),
        current.1.clamp(
            f64::from(options.bounds.top()),
            f64::from((options.bounds.bottom() - pixel).max(options.bounds.top())),
        ),
    );
    let mut annotation = before.clone();
    // 1. 【普通标注】【独立角度】旋转只改变角度，原始矩形和笔迹保持不动
    if handle == SelectionHandle::Rotate {
        let delta = geometry::drag_degrees(
            {
                let center = before.rotation_center();
                (f64::from(center.0), f64::from(center.1))
            },
            start,
            point,
            0.1 / f64::from(options.pixel_scale.max(0.001)),
        )?;
        let degrees = before.rotation_degrees + delta;
        annotation.rotation_degrees = if options.snap_rotation {
            geometry::snapped(degrees)
        } else {
            geometry::normalized(degrees)
        };
        return Some(Change {
            annotation,
            resized: None,
        });
    }
    // 2. 【普通标注】【有界移动】以可见轴对齐选框限制位移，原始坐标和中心同时平移
    if handle == SelectionHandle::Move {
        let delta = translation(
            before,
            (start.0 as f32, start.1 as f32),
            (point.0 as f32, point.1 as f32),
            options.bounds,
        );
        annotation.translate_mut(delta.0, delta.1);
        clamp_points(&mut annotation, options.bounds, pixel, true);
        return Some(Change {
            annotation,
            resized: None,
        });
    }
    // 3. 【普通标注】【反向缩放】先反旋转鼠标，再按原版未旋转矩形计算八方向缩放
    let old = chrome::bounds(before)?;
    let target = resize::resized(
        old,
        handle,
        before.to_unrotated((point.0 as f32, point.1 as f32)),
        options.bounds,
        options.pixel_scale,
        options.keep_aspect,
    )?;
    annotation.bbox = old;
    annotation = annotation.resize_to_bbox(SignedRect {
        left: target.left(),
        top: target.top(),
        right: target.right(),
        bottom: target.bottom(),
    });
    clamp_points(&mut annotation, options.bounds, pixel, false);
    Some(Change {
        annotation,
        resized: Some(target),
    })
}
