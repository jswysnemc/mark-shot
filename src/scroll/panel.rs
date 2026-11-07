//! 【滚动截图】【控制面板】原版六类操作、状态行、细节预览与概览入口

mod capture_marker;
mod draw;
mod layout;
pub mod preview;

pub use draw::{button_icon, draw, is_button_enabled};
pub use layout::{PanelLayout, constrained_layout, hit_test, layout};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelButton {
    ToggleAxis,
    TogglePause,
    Annotate,
    Save,
    Copy,
    Cancel,
}

/// 【滚动截图】【矩形命中】按 rect 与 point 返回点是否位于矩形内，无分配
pub(crate) fn contains(rect: tiny_skia::Rect, point: (f64, f64)) -> bool {
    point.0 >= rect.left() as f64
        && point.0 <= rect.right() as f64
        && point.1 >= rect.top() as f64
        && point.1 <= rect.bottom() as f64
}

/// 【滚动截图】【细边绘制】在 p 中按 rect 与 color 绘制一逻辑像素矩形边线，无返回值
fn border(p: &mut crate::renderer::Painter, rect: tiny_skia::Rect, color: crate::theme::Rgba) {
    p.stroke_path(
        &tiny_skia::PathBuilder::from_rect(rect),
        color,
        &tiny_skia::Stroke {
            width: 1.0,
            ..Default::default()
        },
    );
}
