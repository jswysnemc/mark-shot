//! 【尺子】【会话状态】保存悬停说明与测量标签缓存，未使用尺子时不分配像素缓冲

use tiny_skia::Rect;

use super::labels::InfoLabel;
use super::{Measurement, Point};

#[derive(Clone, Debug, PartialEq)]
pub struct RulerHover {
    pub point: Point,
    pub bounds: Rect,
    pub label: InfoLabel,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LabelSource {
    pub id: u64,
    pub measurement: Measurement,
    pub width: f32,
}

#[derive(Default)]
pub struct RulerState {
    pub entered_from_selection: bool,
    pub hover: Option<RulerHover>,
    pub labels: Vec<InfoLabel>,
    pub label_bounds: Option<Rect>,
    pub label_window: crate::features::annotation_style::window::WindowUnits,
    pub label_projection: crate::editor::canvas_navigation::transform::ViewTransform,
    pub label_sources: Vec<LabelSource>,
}
