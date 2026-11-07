//! 【滚动截图】【应用入口】运行状态、原生输入、真实捕获与结果交接分离

mod capture;
mod completion;
pub(super) mod handoff;
mod input;
mod render;
mod resources;
mod runtime;
mod task;

use crate::scroll::{
    panel::{PanelButton, PanelLayout, preview::PreviewState},
    session::ScrollSession,
};

pub(super) enum Outcome {
    Cancelled,
    Annotate {
        image: tiny_skia::Pixmap,
        monitor_idx: usize,
    },
    Copy {
        png: Vec<u8>,
        scale: f32,
    },
    Saved(std::path::PathBuf),
}

enum PendingOutcome {
    Cancelled,
    Annotate {
        image: shot_stitch::StitchedImage,
        monitor_idx: usize,
    },
    Copy {
        image: shot_stitch::StitchedImage,
        scale: f32,
    },
    Saved(std::path::PathBuf),
}

struct ScrollRun {
    session: ScrollSession,
    layout: PanelLayout,
    hovered: Option<PanelButton>,
    pressed: Option<PanelButton>,
    pointer: (f64, f64),
    axis_drag: Option<input::AxisDrag>,
    overview_drag: Option<f32>,
    preview: PreviewState,
    scale: f32,
    dirty: bool,
    needs_clean_frame: bool,
    panel_limits: Option<(u32, u32)>,
}

/// 【滚动截图】【会话入口】按 conn、state 与 overlay 捕获长图，返回具体用户操作及原始图像
pub(super) async fn capture(
    conn: &crate::backend::session::Session,
    state: &mut crate::editor::EditorState,
    overlay: &mut Box<dyn crate::backend::ScreenOverlay>,
) -> crate::AppResult<Outcome> {
    runtime::run(conn, state, overlay).await
}

#[cfg(test)]
mod tests;
