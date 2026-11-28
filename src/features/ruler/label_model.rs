//! 【尺子】【说明模型】按绘制目标的整数字号准备单行与多行说明

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
    pub(super) wraps: Vec<super::label_flow::Flow>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InfoLabel {
    pub rect: Rect,
    pub unit: f32,
    pub lines: Vec<String>,
    pub font_size: f32,
}

/// 【尺子】【说明文字】按 measurement 返回启用的长度读数或角度，箭头尺和未完成测量无需额外标签
fn text(measurement: Measurement) -> Option<String> {
    match measurement.geometry {
        RulerGeometry::Length { .. } => {
            if measurement.style == super::RulerStyle::Simple {
                None
            } else {
                measurement.info_text()
            }
        }
        RulerGeometry::Angle { points, count: 3 } => Some(format!(
            "{:.1}\u{b0}",
            angle_degrees(points[0], points[1], points[2])
        )),
        RulerGeometry::Angle { .. } => None,
    }
}

/// 【尺子】【说明候选】按 measurement、width 和 projection 返回读数及可选换行方式，不缩小字号
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
    let lines = vec![text];
    let size = super::label_flow::size(&lines, pixels, unit);
    Some(LabelRequest {
        measurement,
        width,
        projection,
        paint_unit: unit,
        paint_size: size,
        unit: (unit / projection.scale) as f32,
        lines,
        wraps: super::label_flow::wraps(measurement, pixels, unit),
        font_size: (f64::from(pixels) / projection.scale) as f32,
        size: (
            (size.0 / projection.scale) as f32,
            (size.1 / projection.scale) as f32,
        ),
    })
}
