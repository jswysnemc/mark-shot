pub mod annotations;
pub mod capture;
pub mod cursor;
pub mod events;
pub mod frozen_pixmap;
pub mod marker_shape;
pub mod pointer_motion;
pub mod popup;
pub mod popup_tree;
pub mod scroll;
pub mod selection;
pub mod text;
pub mod text_input;
pub mod tool_settings;

pub use annotations::*;
pub use capture::*;
pub use cursor::*;
pub use events::*;
pub use marker_shape::MarkerShape;
pub use scroll::*;
pub use selection::*;
pub use text::*;
pub use tool_settings::*;

use tiny_skia::Rect;
#[derive(Clone, Copy, Debug)]
pub struct SignedRect {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl SignedRect {
    pub fn width(&self) -> f32 {
        self.right - self.left
    }
    pub fn height(&self) -> f32 {
        self.bottom - self.top
    }

    pub fn to_rect(&self) -> Option<Rect> {
        Rect::from_ltrb(
            self.left.min(self.right),
            self.top.min(self.bottom),
            self.left.max(self.right),
            self.top.max(self.bottom),
        )
    }
}
pub mod scroll_panel;
#[cfg(target_os = "windows")]
mod windows_output;
