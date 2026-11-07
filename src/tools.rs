pub(crate) mod drawing_space;
pub mod eyedropper;
pub mod filter;
pub mod highlighter;
pub mod laser;
pub mod line;
pub mod magnifier;
pub mod marker;
pub(crate) mod names;
pub mod number;
pub mod numerated_arrow;
pub mod ocr;
pub mod pen;
pub mod pick;
pub mod ruler;
pub mod selection;
mod selection_constraints;
pub mod simple_shapes;
pub mod text;

use crate::editor::EditorState;
use crate::tools::eyedropper::EyedropperTool;
use crate::tools::filter::FilterTool;
use crate::tools::highlighter::HighlighterTool;
use crate::tools::laser::LaserTool;
use crate::tools::magnifier::MagnifierTool;
use crate::tools::marker::MarkerTool;
use crate::tools::number::NumberTool;
use crate::tools::numerated_arrow::NumeratedArrowTool;
use crate::tools::ocr::OcrTool;
use crate::tools::pen::PenTool;
use crate::tools::pick::PickTool;
use crate::tools::ruler::RulerTool;
use crate::tools::selection::SelectionTool;
use crate::tools::simple_shapes::SimpleShapeTool;
use crate::tools::text::TextTool;
use crate::types::annotations::AnnotationShape;
use crate::types::{CursorIcon, MouseButton, SpecialKey, StrokeGroup, ToolSettings};
// ==========================================
// 1. Available Tools
// ==========================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumIter)]
pub enum Tool {
    Selection,
    Rectangle,
    Arrow,
    Circle,
    Pen,
    Highlighter,
    Line,
    Text,
    Pick,
    Eyedropper,
    NumeratedArrow,
    Ocr,
    Redact,
    Laser,
    Marker,
    Ruler,
    Magnifier,
    Number,
}

impl Tool {
    /// 【标注】【绘图类型】返回工具是否拥有可取消的未提交图形；不包含已有标注的拖动或文字编辑
    pub fn is_drawing(self) -> bool {
        matches!(
            self,
            Self::Rectangle
                | Self::Arrow
                | Self::Circle
                | Self::Pen
                | Self::Highlighter
                | Self::Line
                | Self::NumeratedArrow
                | Self::Redact
                | Self::Laser
                | Self::Marker
                | Self::Ruler
                | Self::Magnifier
                | Self::Number
        )
    }
}

// ==========================================
// 2. trait
// ==========================================
pub trait ToolBehavior {
    fn on_button(
        &self,
        state: &mut EditorState,
        button: MouseButton,
        pressed: bool,
        dirty_mask: &mut u32,
    );
    fn on_move(&self, state: &mut EditorState, global: (f64, f64), dirty_mask: &mut u32);
    fn on_activate(&self, _state: &mut EditorState, _dirty_mask: &mut u32) {}
    fn on_deactivate(&self, _state: &mut EditorState, _dirty_mask: &mut u32) {}
    fn on_text(&self, _state: &mut EditorState, _ch: char, _dirty_mask: &mut u32) {}
    fn on_key(&self, _state: &mut EditorState, _key: SpecialKey, _dirty_mask: &mut u32) {}
    // default cursor shape is crosshair
    fn cursor(&self, _state: &EditorState) -> CursorIcon {
        CursorIcon::Crosshair
    }
}

// ==========================================
// 3. Static dispatching
// ==========================================
fn simple_shape(tool: Tool, settings: &ToolSettings) -> SimpleShapeTool {
    SimpleShapeTool {
        make_shape: match tool {
            Tool::Rectangle => |start, end, ts| AnnotationShape::Rectangle {
                start,
                end,
                filled: ts.fill,
            },
            Tool::Circle => |start, end, ts| AnnotationShape::Circle {
                start,
                end,
                filled: ts.fill,
            },
            _ => unreachable!("simple shapes only include rectangles and ellipses"),
        },
        settings: settings.clone(),
        stroke_width: settings.stroke(StrokeGroup::of_tool(tool)),
        snap: matches!(tool, Tool::Line | Tool::Arrow),
    }
}

pub fn dispatch_move(
    tool: Tool,
    state: &mut EditorState,
    global: (f64, f64),
    dirty_mask: &mut u32,
) {
    let global = state.canvas_transform().to_canvas(global);
    match tool {
        Tool::Selection => SelectionTool.on_move(state, global, dirty_mask),
        Tool::Pick => PickTool.on_move(state, global, dirty_mask),
        Tool::Eyedropper => EyedropperTool.on_move(state, global, dirty_mask),
        Tool::Text => TextTool.on_move(state, global, dirty_mask),
        Tool::Pen => PenTool.on_move(state, global, dirty_mask),
        Tool::Highlighter => HighlighterTool.on_move(state, global, dirty_mask),
        Tool::Laser => LaserTool.on_move(state, global, dirty_mask),
        Tool::Marker => MarkerTool.on_move(state, global, dirty_mask),
        Tool::Ruler => RulerTool.on_move(state, global, dirty_mask),
        Tool::Magnifier => MagnifierTool.on_move(state, global, dirty_mask),
        Tool::NumeratedArrow => NumeratedArrowTool.on_move(state, global, dirty_mask),
        Tool::Number => NumberTool.on_move(state, global, dirty_mask),
        Tool::Redact => FilterTool.on_move(state, global, dirty_mask),
        Tool::Ocr => OcrTool.on_move(state, global, dirty_mask),
        Tool::Line | Tool::Arrow => line::LineTool {
            arrow: tool == Tool::Arrow,
        }
        .on_move(state, global, dirty_mask),

        Tool::Rectangle | Tool::Circle => {
            simple_shape(tool, &state.tool_settings).on_move(state, global, dirty_mask);
        }
    }
}

pub fn dispatch_button(
    tool: Tool,
    state: &mut EditorState,
    button: MouseButton,
    pressed: bool,
    dirty_mask: &mut u32,
) {
    match tool {
        Tool::Selection => SelectionTool.on_button(state, button, pressed, dirty_mask),
        Tool::Pick => PickTool.on_button(state, button, pressed, dirty_mask),
        Tool::Eyedropper => EyedropperTool.on_button(state, button, pressed, dirty_mask),
        Tool::Text => TextTool.on_button(state, button, pressed, dirty_mask),
        Tool::Pen => PenTool.on_button(state, button, pressed, dirty_mask),
        Tool::Highlighter => HighlighterTool.on_button(state, button, pressed, dirty_mask),
        Tool::Laser => LaserTool.on_button(state, button, pressed, dirty_mask),
        Tool::Marker => MarkerTool.on_button(state, button, pressed, dirty_mask),
        Tool::Ruler => RulerTool.on_button(state, button, pressed, dirty_mask),
        Tool::Magnifier => MagnifierTool.on_button(state, button, pressed, dirty_mask),
        Tool::NumeratedArrow => NumeratedArrowTool.on_button(state, button, pressed, dirty_mask),
        Tool::Number => NumberTool.on_button(state, button, pressed, dirty_mask),
        Tool::Redact => FilterTool.on_button(state, button, pressed, dirty_mask),
        Tool::Ocr => OcrTool.on_button(state, button, pressed, dirty_mask),
        Tool::Line | Tool::Arrow => line::LineTool {
            arrow: tool == Tool::Arrow,
        }
        .on_button(state, button, pressed, dirty_mask),

        Tool::Rectangle | Tool::Circle => {
            simple_shape(tool, &state.tool_settings).on_button(state, button, pressed, dirty_mask);
        }
    }
}

// runs when a tool becomes the active one (Ocr uses it to start recognition)
// yet its still a `Tool`, since after ocr user can highlight  text and  copy
pub fn dispatch_activate(tool: Tool, state: &mut EditorState, dirty_mask: &mut u32) {
    match tool {
        Tool::Ocr => OcrTool.on_activate(state, dirty_mask),
        Tool::Eyedropper => EyedropperTool.on_activate(state, dirty_mask),
        Tool::Ruler => RulerTool.on_activate(state, dirty_mask),
        _ => {}
    }
}

// for now it only cancels active selection
pub fn dispatch_deactivate(tool: Tool, state: &mut EditorState, dirty_mask: &mut u32) {
    match tool {
        Tool::Eyedropper => EyedropperTool.on_deactivate(state, dirty_mask),
        Tool::Pick => PickTool.on_deactivate(state, dirty_mask),
        Tool::Text => TextTool.on_deactivate(state, dirty_mask),
        Tool::Ocr => OcrTool.on_deactivate(state, dirty_mask),
        Tool::Laser => LaserTool.on_deactivate(state, dirty_mask),
        Tool::Marker => MarkerTool.on_deactivate(state, dirty_mask),
        Tool::Ruler => RulerTool.on_deactivate(state, dirty_mask),
        Tool::Magnifier => MagnifierTool.on_deactivate(state, dirty_mask),
        Tool::Number => NumberTool.on_deactivate(state, dirty_mask),
        Tool::Redact => FilterTool.on_deactivate(state, dirty_mask),
        Tool::Rectangle
        | Tool::Arrow
        | Tool::Circle
        | Tool::Pen
        | Tool::Highlighter
        | Tool::Line
        | Tool::NumeratedArrow => crate::editor::drawing_gesture::cancel(state, dirty_mask),
        _ => {}
    }
}

pub fn dispatch_text(tool: Tool, state: &mut EditorState, ch: char, dirty_mask: &mut u32) {
    if tool == Tool::Text {
        TextTool.on_text(state, ch, dirty_mask)
    }
}

pub fn dispatch_key(tool: Tool, state: &mut EditorState, key: SpecialKey, dirty_mask: &mut u32) {
    match tool {
        Tool::Pick => PickTool.on_key(state, key, dirty_mask),
        Tool::Eyedropper => EyedropperTool.on_key(state, key, dirty_mask),
        Tool::Text => TextTool.on_key(state, key, dirty_mask),
        Tool::Ocr => OcrTool.on_key(state, key, dirty_mask),
        _ => {}
    }
}

pub fn dispatch_cursor(tool: Tool, state: &EditorState) -> CursorIcon {
    match tool {
        Tool::Selection => SelectionTool.cursor(state),
        Tool::Pick => PickTool.cursor(state),
        Tool::Text => TextTool.cursor(state),
        Tool::Ocr => OcrTool.cursor(state),
        Tool::Eyedropper => EyedropperTool.cursor(state),
        Tool::Ruler => RulerTool.cursor(state),
        Tool::Magnifier => MagnifierTool.cursor(state),
        Tool::Pen
        | Tool::Laser
        | Tool::Marker
        | Tool::Highlighter
        | Tool::NumeratedArrow
        | Tool::Number
        | Tool::Rectangle
        | Tool::Arrow
        | Tool::Circle
        | Tool::Line
        | Tool::Redact => CursorIcon::Crosshair,
    }
}
