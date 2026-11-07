//! 【标注选框】【旋转投影】集中读取连接点，外移距离独立于持续画布倍率

use super::chrome::{self, Point};
use crate::types::{Annotation, AnnotationShape};

/// 【标注选框】【间隔投影】按start、end和unit返回反向缩放后的旋转点，保持连接起点与方向，不修改标注
pub fn projected(start: Point, end: Point, unit: f32) -> Point {
    if unit == 1.0 {
        return end;
    }
    (
        start.0 + (end.0 - start.0) * unit,
        start.1 + (end.1 - start.1) * unit,
    )
}

/// 【标注选框】【旋转连接点】按annotation返回与绘制一致的上边起点和旋转圆心，无效或不支持的标注返回空值
pub(crate) fn connector(annotation: &Annotation) -> Option<(Point, Point)> {
    if let Some(points) = annotation.generic_rotation_handle() {
        return Some(points);
    }
    if let Some(line) = annotation.shape.line() {
        let frame = chrome::bounds(annotation).and_then(crate::features::line::drag::frame)?;
        let end = crate::features::line::drag::rotation_point(line, frame);
        return Some(((end.0, frame.top()), end));
    }
    let mut scale = 1.0;
    let (start, end) = match annotation.shape {
        AnnotationShape::Redact { filter } => {
            scale = filter.style_scale;
            let frame = crate::features::filter::drag::frame(filter)?;
            let start = filter.to_visual((chrome::center(frame).0, frame.top()));
            (
                start,
                crate::features::filter::drag::rotation_handle(filter)?,
            )
        }
        AnnotationShape::Magnifier { magnifier } => {
            let frame = super::body_hit::unrotated_bounds(annotation)?;
            let start = magnifier.to_visual((chrome::center(frame).0, frame.top()));
            (
                start,
                magnifier.to_visual((chrome::center(frame).0, frame.top() - 26.0)),
            )
        }
        AnnotationShape::Number { mark } => {
            scale = mark.style_scale;
            let frame = mark.unrotated_bounds(annotation.stroke_width)?;
            let start = mark.to_visual((chrome::center(frame).0, frame.top()));
            (
                start,
                crate::features::number::drag::rotation_handle(mark, annotation.stroke_width)?,
            )
        }
        AnnotationShape::Ruler { measurement } => {
            return crate::features::ruler::window_chrome::connector(
                measurement,
                annotation.stroke_width,
                None,
                Default::default(),
            );
        }
        _ => return None,
    };
    Some((start, projected(start, end, 1.0 / scale.max(f32::EPSILON))))
}

/// 【标注选框】【测试窗口连接点】按annotation和window返回未指定允许范围的生产连接点，供独立几何测试使用
#[cfg(test)]
pub(crate) fn window_connector(
    annotation: &Annotation,
    window: crate::features::annotation_style::window::WindowUnits,
) -> Option<(Point, Point)> {
    window_connector_in_bounds(annotation, window, None)
}

/// 【标注选框】【允许范围连接点】按annotation、window和allowed返回绘制连接点，尺子使用完整窗口数字布局
#[cfg(test)]
pub(crate) fn window_connector_in_bounds(
    annotation: &Annotation,
    window: crate::features::annotation_style::window::WindowUnits,
    allowed: Option<tiny_skia::Rect>,
) -> Option<(Point, Point)> {
    if let AnnotationShape::Ruler { measurement } = annotation.shape {
        return crate::features::ruler::window_chrome::connector(
            measurement,
            annotation.stroke_width,
            allowed,
            window,
        );
    }
    connector(annotation).map(|(start, end)| (start, projected(start, end, window.canvas(1.0))))
}
