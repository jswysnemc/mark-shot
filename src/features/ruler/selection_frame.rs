//! 【尺子】【选中外框】选框只描述屏幕上的编辑控件，不参与测量、吸附或导出

use super::{
    Measurement, Point, RulerGeometry, RulerMode, drag,
    geometry::{rect_center, rotated},
    simple_frame,
    tick_layout::Layout,
};
use crate::{
    editor::canvas_navigation::transform::ViewTransform,
    features::{
        annotation_selection::{chrome, frame},
        annotation_style::window::WindowUnits,
    },
    types::SelectionHandle,
};
use tiny_skia::Rect;

#[derive(Clone, Copy)]
pub(crate) struct Frame {
    pub rect: Rect,
    pivot: Point,
    rotation: f32,
    measurement: Measurement,
    window: WindowUnits,
}

impl Frame {
    /// 【尺子】【选框构造】按 measurement、width、可见范围 allowed 和窗口投影返回不压住尺身的局部操作框
    pub fn new(
        measurement: Measurement,
        width: f32,
        allowed: Option<Rect>,
        window: WindowUnits,
        projection: ViewTransform,
    ) -> Option<Self> {
        let body = measurement.unrotated_bounds()?;
        let (rect, rotation) = match measurement.geometry {
            RulerGeometry::Angle { .. } => {
                let pad = window.canvas(drag::ANGLE_FRAME_PADDING);
                (body.outset(pad, pad)?, 0.0)
            }
            RulerGeometry::Length { rotation, .. } => {
                // 1. 【尺子】【箭头窄框】仅沿测量轴延长两端，横截面仍固定为原来的窗口宽度
                let rect = if let Some(axis) = simple_frame::bounds(measurement, window) {
                    let (_, _, horizontal) = simple_frame::axis(measurement)?;
                    let pad = window.canvas(frame::GAP);
                    if horizontal {
                        axis.outset(pad, 0.0)?
                    } else {
                        axis.outset(0.0, pad)?
                    }
                } else {
                    // 2. 【尺子】【刻度留白】使用当前窗口实际换侧后的数字条带，不把旋转外接矩形再旋转一次
                    let layout = if projection.is_identity() {
                        Layout::new(measurement, width, allowed, window)
                    } else {
                        Layout::in_projection(measurement, width, allowed, projection)
                    }?;
                    frame::outside(layout.local_scale_bounds()?, 0.0, window)?
                };
                (rect, rotation)
            }
        };
        Some(Self {
            rect,
            pivot: rect_center(body),
            rotation,
            measurement,
            window,
        })
    }

    /// 【尺子】【选框四角】返回绕真实尺身轴心旋转后的四角，数字条带不改变旋转中心
    pub fn corners(self) -> [Point; 4] {
        chrome::corners(self.rect).map(|point| rotated(point, self.pivot, self.rotation))
    }

    /// 【尺子】【连线起点】按原有旋转连线的 start 返回外框上的起点，避开箭头尺外置文字且不移动旋转把手
    pub fn stem_start(self, start: Point) -> Point {
        let local = rotated(start, self.pivot, -self.rotation);
        rotated(
            (local.0, local.1.min(self.rect.top())),
            self.pivot,
            self.rotation,
        )
    }

    /// 【尺子】【选框把手】返回实际绘制的缩放点；箭头只保留主轴两端，普通尺按正文大小保留可区分的控制点
    pub fn handles(self) -> impl Iterator<Item = (Point, SelectionHandle)> {
        let axis = simple_frame::axis(self.measurement).map(|(_, _, horizontal)| horizontal);
        let body = self.measurement.unrotated_bounds();
        drag::frame_handles(self.rect)
            .into_iter()
            .filter(move |(_, handle)| {
                use SelectionHandle::{Bottom, Left, Right, Top};
                match axis {
                    Some(true) => matches!(handle, Left | Right),
                    Some(false) => matches!(handle, Top | Bottom),
                    None if self.measurement.mode() == RulerMode::Angle => true,
                    None => body.is_some_and(|body| {
                        drag::distinct(body, *handle, self.window.canvas(drag::HANDLE_TOLERANCE))
                    }),
                }
            })
            .map(move |(point, handle)| (rotated(point, self.pivot, self.rotation), handle))
    }

    /// 【尺子】【外框命中】按画布 point 返回可见把手或外框边缘的移动操作，尺内空白留给工具本身
    pub fn hit(self, point: Point) -> SelectionHandle {
        let radius = self.window.canvas(drag::HANDLE_TOLERANCE);
        let handle = drag::nearest(self.handles(), point, radius * 1.4);
        if handle != SelectionHandle::None {
            return handle;
        }
        let local = rotated(point, self.pivot, -self.rotation);
        if self
            .rect
            .outset(radius, radius)
            .is_some_and(|rect| super::geometry::contains(rect, local))
            && !self
                .rect
                .inset(radius, radius)
                .is_some_and(|rect| super::geometry::contains(rect, local))
        {
            SelectionHandle::Move
        } else {
            SelectionHandle::None
        }
    }
}
