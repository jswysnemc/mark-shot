//! 【截图区域识别】【临时状态】只保存检测结果、悬停候选和当前点击手势

use super::{geometry::Space, task::Worker};
use crate::features::region_detection::{Cursor, Detector, window::Window};
use tiny_skia::Rect;

pub(super) struct Press {
    pub origin: (f64, f64),
    pub candidate: Option<Rect>,
    pub dismissed: bool,
}

#[derive(Default)]
pub(crate) struct State {
    pub(super) space: Option<Space>,
    pub(super) windows: Vec<Window>,
    pub(super) detector: Option<Detector>,
    pub(super) cursor: Cursor,
    pub(crate) hovered: Option<Rect>,
    pub(super) pressed: Option<Press>,
    pub(super) pointer_valid: bool,
    pub(super) worker: Option<Worker>,
}
