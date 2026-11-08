//! 【标注】【数据类型】持久标注、形状和拖动快照，不承担编辑事务

mod bounds;
mod rotation;
mod style;
mod text_frame;
mod transform;
pub use bounds::arrow_head;

use crate::types::{MarkerShape, SelectionHandle};
use tiny_skia::{Color, Rect};

#[derive(Clone, PartialEq, Debug)]
pub enum AnnotationShape {
    ColorSample {
        sample: crate::features::color_sample::ColorSample,
    },
    Number {
        mark: crate::features::number::NumberMark,
    },
    NumeratedArrow {
        start: (f32, f32),
        end: (f32, f32),
        number: u32,
    },
    Arrow {
        line: crate::features::line::LineSkeleton,
        style: crate::features::arrow::ArrowStyle,
        line_style: crate::features::line::LineStyle,
    },
    Rectangle {
        start: (f32, f32),
        end: (f32, f32),
        filled: bool,
    },
    Marker {
        start: (f32, f32),
        end: (f32, f32),
        kind: MarkerShape,
        filled: bool,
    },
    Ruler {
        measurement: crate::features::ruler::Measurement,
    },
    Magnifier {
        magnifier: crate::features::magnifier::Magnifier,
    },
    Circle {
        start: (f32, f32),
        end: (f32, f32),
        filled: bool,
    },
    Line {
        line: crate::features::line::LineSkeleton,
        style: crate::features::line::LineStyle,
    },
    Redact {
        filter: crate::features::filter::RegionFilter,
    },
    Pen {
        points: Vec<(f32, f32)>,
    },
    Highlighter {
        points: Vec<(f32, f32)>,
        flat_ends: [bool; 2],
    },
    Text {
        frame: crate::features::text::TextFrame,
        appearance: crate::features::text::TextAppearance,
        start: (f32, f32),
        content: String,
        font_size: f32,
        bold: bool,
        italic: bool,
        outline: Option<Color>,
    },
}

pub struct AnnDragState {
    pub handle: SelectionHandle,
    pub start_global: (f64, f64),
    pub prev_global: (f64, f64),
    /// 开始拖动时的标注快照
    pub orig: Annotation,
    pub orig_index: usize,
}

impl AnnotationShape {
    /// 【标注】【起点读取】返回形状的首个逻辑坐标；空笔迹返回原点
    pub fn start_point(&self) -> (f32, f32) {
        match self {
            AnnotationShape::ColorSample { sample } => sample.source,
            AnnotationShape::Number { mark } => mark.bubble,
            AnnotationShape::NumeratedArrow { start, .. } => *start,
            AnnotationShape::Arrow { line, .. } => line.start,
            AnnotationShape::Rectangle { start, .. } => *start,
            AnnotationShape::Marker { start, .. } => *start,
            AnnotationShape::Ruler { measurement } => measurement.start(),
            AnnotationShape::Magnifier { magnifier } => magnifier.source,
            AnnotationShape::Circle { start, .. } => *start,
            AnnotationShape::Line { line, .. } => line.start,
            AnnotationShape::Redact { filter } => filter.start,
            AnnotationShape::Pen { points } | AnnotationShape::Highlighter { points, .. } => {
                points.first().copied().unwrap_or((0.0, 0.0))
            }
            AnnotationShape::Text { start, .. } => *start,
        }
    }

    /// 【标注】【线条骨架】返回直线或箭头共用骨架；其他形状返回空值
    pub fn line(&self) -> Option<&crate::features::line::LineSkeleton> {
        match self {
            Self::Arrow { line, .. } | Self::Line { line, .. } => Some(line),
            _ => None,
        }
    }

    /// 【标注】【线条编辑】返回可修改的直线或箭头骨架；调用方负责边界和历史事务
    pub fn line_mut(&mut self) -> Option<&mut crate::features::line::LineSkeleton> {
        match self {
            Self::Arrow { line, .. } | Self::Line { line, .. } => Some(line),
            _ => None,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Annotation {
    pub id: u64,
    /// 【标注样式】【原图比例】一个原图像素在当前坐标中的尺寸，导出换坐标时同步缩放，交互缩放保留此比例
    pub style_scale: f32,
    /// 【普通标注】【旋转角度】独立于原始几何保存顺时针角度，专用工具继续使用自身旋转模型
    pub rotation_degrees: f64,
    /// 【普通标注】【冻结画布】保存正文坐标所在画布，边缘选框先裁切再计算轴心，测试或独立几何可省略
    pub frozen_bounds: Option<Rect>,
    pub shape: AnnotationShape,
    pub color: Color,
    pub shadow_color: Color,
    pub stroke_width: f32,
    pub bbox: Rect,
}
