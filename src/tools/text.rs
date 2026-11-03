mod keyboard;
pub use keyboard::apply_key_to_editor;
mod behavior;
mod layout;
use layout::sync_content_from_editor;
pub use layout::{
    ensure_text_editor, reflow_text_editor, shape_entire_buffer, text_bounds,
    update_text_bbox_inline,
};
mod background;
mod render;
pub use render::{render_text_annotation, render_text_annotation_with_caret};
mod selection;
pub mod session_input;
use selection::extract_selected_text;

use crate::editor::annotation_drag::{apply_annotation_drag, handle_hit_test_for_annotation};
use crate::editor::{DamageZone, EditorState};
use crate::theme::color;
use crate::tools::ToolBehavior;
use crate::types::{Annotation, AnnotationShape, CursorIcon, MouseButton, SpecialKey};
use crate::utils::cursor_for_handle;
use cosmic_text::{
    Action, Attrs, Buffer, Command, Edit, Editor, Family, Metrics, Motion, Selection, SwashCache,
};
use log::info;
use tiny_skia::{
    BlendMode, FillRule, LineJoin, Paint, Path, PathBuilder, Pixmap, PixmapMut, PixmapPaint, Rect,
    Stroke, Transform,
};

pub struct TextTool;

// ── key -> Action ─────────────────────────────────────────────────────────────

// I don't know, if its a tiny-skia thing, but the stroke runs in the middle of the letter edge, half outside, and half inside not visible
pub fn text_outline_stroke_width(font_size: f32) -> f32 {
    font_size * crate::config::get().annotations.text.outline_width.max(0.0) * 2.0
}

fn min_width() -> f32 {
    crate::config::get().annotations.text.min_width.max(0.0)
}

// small helper
fn tiny_skia_to_cosmic(c: tiny_skia::Color) -> cosmic_text::Color {
    cosmic_text::Color::rgba(
        (c.red() * 255.0).round() as u8,
        (c.green() * 255.0).round() as u8,
        (c.blue() * 255.0).round() as u8,
        (c.alpha() * 255.0).round() as u8,
    )
}
