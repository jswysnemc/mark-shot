//! 【尺子渲染】【布局接入】生产绘制和局部剔除共用允许范围及窗口字体

use super::Painter;
use crate::features::{
    annotation_style::window::WindowUnits,
    ruler::{Measurement, tick_layout::Layout},
};
use tiny_skia::Rect;

/// 【尺子渲染】【正文布局】按painter、measurement和width返回当前允许范围的完整数字布局
pub(crate) fn layout(
    painter: &Painter<'_>,
    measurement: Measurement,
    width: f32,
) -> Option<Layout> {
    if !painter.ruler_projection().is_identity() {
        return Layout::in_projection(
            measurement,
            width,
            painter.drawing_bounds(),
            painter.ruler_projection(),
        );
    }
    Layout::new(
        measurement,
        width,
        painter.drawing_bounds(),
        painter.window_units(),
    )
}

/// 【尺子渲染】【控件连接】按painter和annotation返回实际窗口投影中的连线和把手，非尺子返回空值
pub(crate) fn connector(
    painter: &Painter<'_>,
    annotation: &crate::types::Annotation,
) -> Option<((f32, f32), (f32, f32))> {
    let crate::types::AnnotationShape::Ruler { measurement } = annotation.shape else {
        return None;
    };
    if !painter.ruler_projection().is_identity() {
        return crate::features::ruler::window_chrome::in_projection(
            measurement,
            annotation.stroke_width,
            painter.drawing_bounds(),
            painter.ruler_projection(),
        );
    }
    crate::features::ruler::window_chrome::connector(
        measurement,
        annotation.stroke_width,
        painter.drawing_bounds(),
        painter.window_units(),
    )
}

/// 【尺子渲染】【实际范围】按measurement、width、allowed和window返回正文含数字描边的范围，角度尺沿用圆弧范围
pub(crate) fn bounds(
    measurement: Measurement,
    width: f32,
    allowed: Option<Rect>,
    window: WindowUnits,
    projection: crate::editor::canvas_navigation::transform::ViewTransform,
) -> Option<Rect> {
    let unit = measurement.unit(width);
    let layout = if projection.is_identity() {
        Layout::new(measurement, width, allowed, window)
    } else {
        Layout::in_projection(measurement, width, allowed, projection)
    };
    layout
        .and_then(Layout::scale_bounds)
        .or_else(|| measurement.visual_bounds(unit))?
        .outset(
            window.canvas(0.8).max(unit) + window.canvas(2.0),
            window.canvas(0.8).max(unit) + window.canvas(2.0),
        )
}
