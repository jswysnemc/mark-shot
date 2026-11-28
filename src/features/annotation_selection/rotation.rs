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
    window_connector_in_bounds(annotation, Default::default(), None)
}

/// 【标注选框】【窗口连线】按 annotation、window 和允许范围 allowed 返回真实控件位置，连线只换算一次窗口单位
pub(crate) fn window_connector_in_bounds(
    annotation: &Annotation,
    window: crate::features::annotation_style::window::WindowUnits,
    allowed: Option<tiny_skia::Rect>,
) -> Option<(Point, Point)> {
    if annotation.uses_generic_rotation() {
        return annotation.generic_rotation_handle_in_window(window);
    }
    if let Some(points) = super::region_frame::connector(annotation, window) {
        return Some(points);
    }
    if let Some(line) = annotation.shape.line() {
        let frame = chrome::bounds(annotation).and_then(crate::features::line::drag::frame)?;
        let end = crate::features::line::drag::rotation_point(line, frame);
        let start = (end.0, frame.top());
        return Some((start, projected(start, end, window.canvas(1.0))));
    }
    match annotation.shape {
        AnnotationShape::Number { mark } => {
            let frame = mark.unrotated_bounds(annotation.stroke_width)?;
            let start = mark.to_visual((chrome::center(frame).0, frame.top()));
            let end =
                crate::features::number::drag::rotation_handle(mark, annotation.stroke_width)?;
            Some((
                start,
                projected(
                    start,
                    end,
                    window.canvas(1.0) / mark.style_scale.max(f32::EPSILON),
                ),
            ))
        }
        AnnotationShape::Ruler { measurement } => crate::features::ruler::window_chrome::connector(
            measurement,
            annotation.stroke_width,
            allowed,
            window,
        ),
        _ => None,
    }
}

/// 【标注选框】【测试窗口连接点】按annotation和window返回未指定允许范围的生产连接点，供独立几何测试使用
#[cfg(test)]
pub(crate) fn window_connector(
    annotation: &Annotation,
    window: crate::features::annotation_style::window::WindowUnits,
) -> Option<(Point, Point)> {
    window_connector_in_bounds(annotation, window, None)
}
