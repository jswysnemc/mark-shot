//! 【区域标注】【编辑外框】滤镜和放大镜的交互留白独立于遮罩和采样矩形

use super::{chrome, frame};
use crate::{
    features::annotation_style::window::WindowUnits,
    types::{Annotation, AnnotationShape},
};
use tiny_skia::Rect;

/// 【区域标注】【联合选框】按 annotation 与 window 返回未旋转操作框，非区域标注返回空值
pub(crate) fn bounds(annotation: &Annotation, window: WindowUnits) -> Option<Rect> {
    let body = match annotation.shape {
        AnnotationShape::Redact { filter } => filter.rect()?,
        AnnotationShape::Magnifier { magnifier } => magnifier.unrotated_bounds()?,
        _ => return None,
    };
    frame::outside(body, 0.0, window)
}

/// 【区域标注】【视觉映射】按 annotation 的真实轴心旋转 point，非区域标注保持原点
pub(crate) fn visual(annotation: &Annotation, point: chrome::Point) -> chrome::Point {
    match annotation.shape {
        AnnotationShape::Redact { filter } => filter.to_visual(point),
        AnnotationShape::Magnifier { magnifier } => magnifier.to_visual(point),
        _ => point,
    }
}

/// 【区域标注】【旋转连线】按 annotation、window 返回外框上方的连接点和旋转圆心，长度固定为二十六窗口像素
pub(crate) fn connector(
    annotation: &Annotation,
    window: WindowUnits,
) -> Option<(chrome::Point, chrome::Point)> {
    let (top, knob) = stem(bounds(annotation, window)?, window);
    Some((visual(annotation, top), visual(annotation, knob)))
}

/// 【区域标注】【局部连线】按外框 rect 和窗口单位 window 返回未旋转的连线两端
pub(crate) fn stem(rect: Rect, window: WindowUnits) -> (chrome::Point, chrome::Point) {
    let top = (chrome::center(rect).0, rect.top());
    (top, (top.0, top.1 - window.canvas(26.0)))
}

/// 【区域标注】【控件重绘】按 annotation、window 返回包含正文、旋转后的外框和把手的画布范围
pub(crate) fn damage(annotation: &Annotation, window: WindowUnits) -> Option<Rect> {
    let corners =
        chrome::corners(bounds(annotation, window)?).map(|point| visual(annotation, point));
    let (_, knob) = connector(annotation, window)?;
    let points = [corners[0], corners[1], corners[2], corners[3], knob];
    let padding = window.canvas(8.0);
    let body = annotation
        .bbox
        .outset(annotation.visual_pad(), annotation.visual_pad())?;
    crate::features::ruler::geometry::points_bounds(&points, padding)?
        .outset(padding, padding)?
        .join(&body)
}
