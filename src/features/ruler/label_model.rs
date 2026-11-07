//! 【尺子】【说明模型】固定单行测量文字，按当前绘制目标的整数像素字号计算完整尺寸

use super::{Measurement, RulerGeometry, geometry::angle_degrees};
use crate::editor::canvas_navigation::transform::ViewTransform;
use tiny_skia::Rect;

#[derive(Clone, Debug, PartialEq)]
pub struct LabelRequest {
    pub measurement: Measurement,
    pub unit: f32,
    pub lines: Vec<String>,
    pub font_size: f32,
    pub size: (f32, f32),
    pub(crate) width: f32,
    pub(crate) projection: ViewTransform,
    pub(crate) paint_unit: f64,
    pub(crate) paint_size: (f64, f64),
}

#[derive(Clone, Debug, PartialEq)]
pub struct InfoLabel {
    pub rect: Rect,
    pub unit: f32,
    pub lines: Vec<String>,
    pub font_size: f32,
}

/// 【尺子】【完整单行】按measurement返回原版说明文字，三点角度即使退化也显示读数，未完成测量返回空值
fn text(measurement: Measurement) -> Option<String> {
    match measurement.geometry {
        RulerGeometry::Length { .. } => measurement.info_text(),
        RulerGeometry::Angle { points, count: 3 } => Some(format!(
            "{:.1}\u{b0}",
            angle_degrees(points[0], points[1], points[2])
        )),
        RulerGeometry::Angle { .. } => None,
    }
}

/// 【尺子】【完整尺寸】按measurement、width和projection返回单行说明请求，不缩小字号或重写说明内容
pub(super) fn prepare(
    measurement: Measurement,
    width: f32,
    projection: ViewTransform,
) -> Option<LabelRequest> {
    let text = text(measurement)?;
    let unit = (1.0 + (f64::from(width * measurement.pixel_scale) - 3.0) / 9.0).clamp(0.75, 3.5)
        * measurement.style_scale
        * projection.scale;
    let pixels = (13.0 * unit).round().max(6.0) as f32;
    let metrics = crate::fonts::ruler::measure(&text, pixels);
    let size = (
        f64::from(metrics.width.ceil()) + 20.0 * unit,
        f64::from(metrics.height.ceil()) + 10.0 * unit,
    );
    Some(LabelRequest {
        measurement,
        width,
        projection,
        paint_unit: unit,
        paint_size: size,
        unit: (unit / projection.scale) as f32,
        lines: vec![text],
        font_size: (f64::from(pixels) / projection.scale) as f32,
        size: (
            (size.0 / projection.scale) as f32,
            (size.1 / projection.scale) as f32,
        ),
    })
}
