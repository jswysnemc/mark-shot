//! 【尺子】【窗口连接点】正文刻度布局和选中框共用顶部留量，连线固定二十窗口像素

use super::{Measurement, Point, RulerGeometry, geometry::rect_center, tick_layout::Layout};
use crate::features::annotation_style::window::WindowUnits;
use tiny_skia::Rect;

/// 【尺子】【窗口旋转】按measurement、width、allowed与window返回连线起点及旋转圆心，计算使用实际窗口字体和完整数字条带
pub fn connector(
    measurement: Measurement,
    width: f32,
    allowed: Option<Rect>,
    window: WindowUnits,
) -> Option<(Point, Point)> {
    let clearance = if matches!(measurement.geometry, RulerGeometry::Length { .. }) {
        Layout::new(measurement, width, allowed, window)?.top_reach()
    } else {
        window.canvas(14.0)
    };
    connect(measurement, clearance, window)
}

/// 【尺子】【窗口投影连接】按measurement、width、allowed和projection计算当前窗口布局，再返回稳定坐标连接点
pub(crate) fn in_projection(
    measurement: Measurement,
    width: f32,
    allowed: Option<Rect>,
    projection: crate::editor::canvas_navigation::transform::ViewTransform,
) -> Option<(Point, Point)> {
    let window = WindowUnits::new(1.0 / projection.scale as f32);
    let clearance = if matches!(measurement.geometry, RulerGeometry::Length { .. }) {
        Layout::in_projection(measurement, width, allowed, projection)?.top_reach()
    } else {
        window.canvas(14.0)
    };
    connect(measurement, clearance, window)
}

/// 【尺子】【连接组装】按measurement、clearance和window返回连线两端，平直尺沿用原版方向退化阈值
fn connect(
    measurement: Measurement,
    clearance: f32,
    window: WindowUnits,
) -> Option<(Point, Point)> {
    let bounds = measurement.unrotated_bounds()?;
    let center = rect_center(bounds);
    let rotation = match measurement.geometry {
        RulerGeometry::Length { rotation, .. } => rotation,
        _ => 0.0,
    };
    let (sin, cos) = f64::from(rotation).to_radians().sin_cos();
    let half_height = f64::from(bounds.height()) / 2.0;
    let top = (
        f64::from(center.0) + sin * half_height,
        f64::from(center.1) - cos * half_height,
    );
    // 1. 【尺子】【平直方向】原版上边距轴心不超过0.1窗口像素时使用竖直向上方向
    let direction = if window.window(half_height as f32) <= 0.1 {
        (0.0, -1.0)
    } else {
        (sin, -cos)
    };
    let start = (
        top.0 + direction.0 * f64::from(clearance),
        top.1 + direction.1 * f64::from(clearance),
    );
    let end = (
        start.0 + direction.0 * f64::from(window.canvas(20.0)),
        start.1 + direction.1 * f64::from(window.canvas(20.0)),
    );
    Some((
        (start.0 as f32, start.1 as f32),
        (end.0 as f32, end.1 as f32),
    ))
}
