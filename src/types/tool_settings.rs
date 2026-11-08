use tiny_skia::Color;

use crate::theme::Rgba;
use crate::tools::Tool;
use crate::types::AnnotationShape;

pub use crate::features::filter::STRENGTH;

pub fn default_color() -> Rgba {
    crate::editor::saved::get()
        .color
        .unwrap_or(crate::config::get().tools.color)
}

/// Tools that share one stroke width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrokeGroup {
    Rectangle,
    Circle,
    Arrow,
    Line,
    Numbered,
    Freehand,
    Laser,
    Marker,
    Ruler,
    Magnifier,
    Number,
    Highlighter,
}

impl StrokeGroup {
    pub const ALL: [Self; 12] = [
        Self::Rectangle,
        Self::Circle,
        Self::Arrow,
        Self::Line,
        Self::Numbered,
        Self::Freehand,
        Self::Laser,
        Self::Marker,
        Self::Ruler,
        Self::Magnifier,
        Self::Number,
        Self::Highlighter,
    ];

    /// name in the saved state file
    pub fn key(self) -> &'static str {
        match self {
            Self::Rectangle => "rectangle",
            Self::Circle => "circle",
            Self::Arrow => "arrow",
            Self::Line => "line",
            Self::Numbered => "numbered",
            Self::Freehand => "freehand",
            Self::Laser => "laser",
            Self::Marker => "marker",
            Self::Ruler => "ruler",
            Self::Magnifier => "magnifier",
            Self::Number => "number",
            Self::Highlighter => "highlighter",
        }
    }

    /// tools without a stroke width never show the stepper, so the fallback is never read
    pub fn of_tool(tool: Tool) -> Self {
        match tool {
            Tool::Circle => Self::Circle,
            Tool::Arrow => Self::Arrow,
            Tool::Line => Self::Line,
            Tool::NumeratedArrow => Self::Numbered,
            Tool::Number => Self::Number,
            Tool::Pen => Self::Freehand,
            Tool::Highlighter => Self::Highlighter,
            Tool::Laser => Self::Laser,
            Tool::Marker => Self::Marker,
            Tool::Ruler => Self::Ruler,
            Tool::Magnifier => Self::Magnifier,
            _ => Self::Rectangle,
        }
    }

    pub fn of_shape(shape: &AnnotationShape) -> Self {
        match shape {
            AnnotationShape::Circle { .. } => Self::Circle,
            AnnotationShape::Arrow { .. } => Self::Arrow,
            AnnotationShape::Line { .. } => Self::Line,
            AnnotationShape::NumeratedArrow { .. } => Self::Numbered,
            AnnotationShape::Number { .. } => Self::Number,
            AnnotationShape::Pen { .. } => Self::Freehand,
            AnnotationShape::Highlighter { .. } => Self::Highlighter,
            AnnotationShape::Marker { .. } => Self::Marker,
            AnnotationShape::Ruler { .. } => Self::Ruler,
            AnnotationShape::Magnifier { .. } => Self::Magnifier,
            _ => Self::Rectangle,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolSettings {
    strokes: [f32; StrokeGroup::ALL.len()],
    pub font_size: f32,
    pub text_family: String,
    pub text_background: Color,
    pub strength: f32,
    pub filter_mode: crate::features::filter::FilterMode,
    pub bold: bool,
    pub italic: bool,
    pub outline: bool,
    pub outline_color: Color,
    pub fill: bool,
    pub corner_radius: f32,
    pub fixed_ratio: bool,
    pub marker_shape: crate::types::MarkerShape,
    pub marker_filled: bool,
    pub ruler_mode: crate::features::ruler::RulerMode,
    pub magnifier_shape: crate::features::magnifier::MagnifierShape,
    pub magnifier_zoom: f32,
    pub number_style: crate::features::number::NumberStyle,
    pub arrow_style: crate::features::arrow::ArrowStyle,
    pub arrow_line_style: crate::features::line::LineStyle,
    pub line_style: crate::features::line::LineStyle,
    pub color: Color,
}

impl ToolSettings {
    pub fn stroke(&self, group: StrokeGroup) -> f32 {
        self.strokes[group as usize]
    }

    pub fn set_stroke(&mut self, group: StrokeGroup, width: f32) {
        self.strokes[group as usize] = width;
    }
}

impl Default for ToolSettings {
    fn default() -> Self {
        let tools = &crate::config::get().tools;
        let saved = crate::editor::saved::get();
        let strokes = StrokeGroup::ALL.map(|group| {
            let configured = match group {
                StrokeGroup::Laser => tools.laser.width,
                StrokeGroup::Marker => tools.marker.width,
                StrokeGroup::Ruler => tools.ruler.width,
                StrokeGroup::Magnifier => tools.magnifier.width,
                StrokeGroup::Number => tools.number.width,
                StrokeGroup::Arrow => tools.arrow.width,
                StrokeGroup::Line => tools.line.width,
                _ => tools.stroke.width,
            };
            let width = saved.stroke(group).unwrap_or(configured);
            if group == StrokeGroup::Number {
                crate::features::number::clamped_width(width, 1.0)
            } else if matches!(group, StrokeGroup::Arrow | StrokeGroup::Line) {
                crate::features::line::clamped_width(width, 1.0)
            } else {
                width.clamp(tools.stroke.min, tools.stroke.max)
            }
        });
        Self {
            strokes,
            text_family: saved
                .text_family
                .clone()
                .unwrap_or_else(|| "Sans Serif".into()),
            text_background: saved.text_background.unwrap_or(Rgba(0, 0, 0, 0)).color(),
            font_size: saved
                .font_size
                .unwrap_or(tools.font.size)
                .clamp(tools.font.min, tools.font.max),
            strength: crate::features::filter::clamped_strength(
                saved.strength.unwrap_or(tools.redact.strength),
            ),
            filter_mode: saved.filter_mode.unwrap_or_else(|| {
                tools
                    .redact
                    .mode
                    .or_else(|| {
                        tools
                            .redact
                            .pixelate
                            .map(crate::features::filter::FilterMode::from_pixelate)
                    })
                    .unwrap_or_default()
            }),
            bold: saved.bold.unwrap_or(tools.font.bold),
            italic: saved.italic.unwrap_or(tools.font.italic),
            outline: saved.outline.unwrap_or(tools.font.outline),
            outline_color: saved
                .outline_color
                .unwrap_or(tools.font.outline_color)
                .color(),
            fill: saved.fill.unwrap_or(tools.fill),
            corner_radius: crate::features::shape::radius(saved.corner_radius.unwrap_or(0.0)),
            fixed_ratio: saved.fixed_ratio.unwrap_or(false),
            marker_shape: saved.marker_shape.unwrap_or(tools.marker.shape),
            marker_filled: saved.marker_filled.unwrap_or(tools.marker.filled),
            ruler_mode: saved.ruler_mode.unwrap_or(tools.ruler.mode),
            magnifier_shape: saved.magnifier_shape.unwrap_or(tools.magnifier.shape),
            magnifier_zoom: crate::features::magnifier::clamped_zoom(
                saved.magnifier_zoom.unwrap_or(tools.magnifier.zoom),
            ),
            number_style: saved.number_style.unwrap_or(tools.number.style),
            arrow_style: saved.arrow_style.unwrap_or(tools.arrow.style),
            arrow_line_style: saved.arrow_line_style.unwrap_or(tools.arrow.line_style),
            line_style: saved.line_style.unwrap_or(tools.line.style),
            color: default_color().color(),
        }
    }
}
