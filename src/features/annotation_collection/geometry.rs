//! 【标注多选】【共同边界】按原图可见范围组合成员选框，框选采用相交而非完整包含

use crate::{
    features::annotation_selection::chrome,
    types::{Annotation, AnnotationShape, SelectionHandle},
};
use tiny_skia::Rect;

/// 【标注多选】【原始范围】按annotation与canvas返回生产未旋转选择范围，尺子保持自身边界，序号采用气泡矩形
pub(crate) fn unrotated_bounds(annotation: &Annotation, canvas: Rect) -> Option<Rect> {
    let bounds = match &annotation.shape {
        AnnotationShape::Arrow { line, .. } | AnnotationShape::Line { line, .. } => {
            let points: Vec<_> = line.points().collect();
            crate::features::ruler::geometry::points_bounds(&points, f32::EPSILON)?
                .outset(annotation.stroke_width, annotation.stroke_width)?
        }
        AnnotationShape::Number { mark } => {
            let radius = mark.bubble_radius(annotation.stroke_width);
            Rect::from_xywh(
                mark.bubble.0 - radius,
                mark.bubble.1 - radius,
                radius * 2.0,
                radius * 2.0,
            )?
        }
        AnnotationShape::Magnifier { magnifier } => magnifier
            .unrotated_bounds()?
            .outset(annotation.stroke_width, annotation.stroke_width)?,
        AnnotationShape::Redact { filter } => filter.rect()?,
        AnnotationShape::Ruler { measurement } => return measurement.unrotated_bounds(),
        _ => chrome::bounds(annotation)?,
    };
    bounds
        .intersect(&canvas)
        .filter(|bounds| !bounds.is_empty())
}

/// 【标注多选】【成员轴心】按annotation与canvas返回生产自身旋转中心，序号使用气泡圆心，其余使用裁切后的原始选框中心
pub(crate) fn rotation_center(annotation: &Annotation, canvas: Rect) -> Option<chrome::Point> {
    if let AnnotationShape::Number { mark } = annotation.shape {
        Some(mark.bubble)
    } else {
        unrotated_bounds(annotation, canvas).map(chrome::center)
    }
}

/// 【标注多选】【成员范围】按annotation与canvas返回原版旋转后可见选框，不包括正文阴影，完全不可见返回空值
pub fn visible_bounds(annotation: &Annotation, canvas: Rect) -> Option<Rect> {
    let before = unrotated_bounds(annotation, canvas)?;
    let degrees = match &annotation.shape {
        AnnotationShape::Number { mark } => f64::from(mark.rotation.to_degrees()),
        AnnotationShape::Magnifier { magnifier } => f64::from(magnifier.rotation.to_degrees()),
        AnnotationShape::Redact { filter } => f64::from(filter.rotation.to_degrees()),
        AnnotationShape::Ruler { measurement } => match measurement.geometry {
            crate::features::ruler::RulerGeometry::Length { rotation, .. } => f64::from(rotation),
            _ => return Some(before),
        },
        _ => annotation.rotation_degrees,
    };
    let bounds = crate::features::annotation_rotation::geometry::bounds(
        before,
        rotation_center(annotation, canvas)?,
        degrees,
    );
    bounds
        .intersect(&canvas)
        .filter(|bounds| !bounds.is_empty())
}

/// 【标注多选】【联合边界】按annotations、ids与canvas返回有效成员可见选框的联合范围，空集合返回空值
pub fn selected_bounds(annotations: &[Annotation], ids: &[u64], canvas: Rect) -> Option<Rect> {
    annotations
        .iter()
        .filter(|annotation| ids.contains(&annotation.id))
        .filter_map(|annotation| visible_bounds(annotation, canvas))
        .reduce(|area, next| area.join(&next).unwrap_or(area))
}

/// 【标注多选】【相交框选】按annotations、marquee、canvas与pixel_scale返回图层顺序编号，框两边不足两个原图像素返回空集合
pub fn in_rect(
    annotations: &[Annotation],
    marquee: Rect,
    canvas: Rect,
    pixel_scale: f32,
) -> Vec<u64> {
    if !pixel_scale.is_finite()
        || pixel_scale <= 0.0
        || marquee.width() * pixel_scale < 2.0
        || marquee.height() * pixel_scale < 2.0
    {
        return Vec::new();
    }
    annotations
        .iter()
        .filter_map(|annotation| {
            visible_bounds(annotation, canvas)?
                .intersect(&marquee)
                .filter(|area| !area.is_empty())
                .map(|_| annotation.id)
        })
        .collect()
}

/// 【标注多选】【旋转位置】按bounds返回上边中点及外移26逻辑像素的群组旋转点
pub fn rotation_handle(bounds: Rect) -> (chrome::Point, chrome::Point) {
    let top = (chrome::center(bounds).0, bounds.top());
    (top, (top.0, top.1 - 26.0))
}

/// 【标注多选】【共同命中】按bounds与point返回群组旋转、缩放或移动控制点，先命中旋转再按原版边界容差判断
pub fn hit_test(bounds: Rect, point: (f64, f64)) -> SelectionHandle {
    if !point.0.is_finite() || !point.1.is_finite() {
        return SelectionHandle::None;
    }
    let rotate = rotation_handle(bounds).1;
    if (point.0 - f64::from(rotate.0)).hypot(point.1 - f64::from(rotate.1)) <= 8.0 * 1.4 {
        return SelectionHandle::Rotate;
    }
    let near_left = (point.0 - f64::from(bounds.left())).abs() <= 10.0;
    let near_right = (point.0 - f64::from(bounds.right())).abs() <= 10.0;
    let near_top = (point.1 - f64::from(bounds.top())).abs() <= 10.0;
    let near_bottom = (point.1 - f64::from(bounds.bottom())).abs() <= 10.0;
    use SelectionHandle::*;
    match (near_left, near_right, near_top, near_bottom) {
        (true, _, true, _) => TopLeft,
        (_, true, true, _) => TopRight,
        (true, _, _, true) => BottomLeft,
        (_, true, _, true) => BottomRight,
        (true, _, _, _) => Left,
        (_, true, _, _) => Right,
        (_, _, true, _) => Top,
        (_, _, _, true) => Bottom,
        _ if point.0 >= f64::from(bounds.left()) - 10.0
            && point.0 <= f64::from(bounds.right()) + 10.0
            && point.1 >= f64::from(bounds.top()) - 10.0
            && point.1 <= f64::from(bounds.bottom()) + 10.0 =>
        {
            Move
        }
        _ => None,
    }
}
