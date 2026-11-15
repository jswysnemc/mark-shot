//! 【标注】【坐标变换】处理导出缩放、逻辑平移与按边界调整尺寸

use super::{Annotation, AnnotationShape};
use crate::types::SignedRect;
use tiny_skia::Rect;

impl Annotation {
    /// 【标注】【比例映射】按比例 s 缩放坐标、字号与笔宽；返回新的标注，不修改原标注
    pub fn scaled(&self, s: f32) -> Annotation {
        let p = |(x, y): (f32, f32)| (x * s, y * s);
        let shape = match &self.shape {
            AnnotationShape::ColorSample { sample } => AnnotationShape::ColorSample {
                sample: sample.scaled(s),
            },
            AnnotationShape::Number { mark } => AnnotationShape::Number {
                mark: mark.scaled(s),
            },
            AnnotationShape::Magnifier { magnifier } => AnnotationShape::Magnifier {
                magnifier: magnifier.scaled(s),
            },
            AnnotationShape::Ruler { measurement } => AnnotationShape::Ruler {
                measurement: measurement.scaled(s),
            },
            AnnotationShape::NumeratedArrow { start, end, number } => {
                AnnotationShape::NumeratedArrow {
                    start: p(*start),
                    end: p(*end),
                    number: *number,
                }
            }
            AnnotationShape::Arrow {
                line,
                style,
                line_style,
            } => AnnotationShape::Arrow {
                line: line.scaled(s),
                style: *style,
                line_style: *line_style,
            },
            AnnotationShape::Rectangle {
                start,
                end,
                filled,
                radius,
                fixed_ratio,
            } => AnnotationShape::Rectangle {
                start: p(*start),
                end: p(*end),
                filled: *filled,
                radius: *radius,
                fixed_ratio: *fixed_ratio,
            },
            AnnotationShape::Marker {
                start,
                end,
                kind,
                filled,
                radius,
            } => AnnotationShape::Marker {
                start: p(*start),
                end: p(*end),
                kind: *kind,
                filled: *filled,
                radius: *radius,
            },
            AnnotationShape::Circle {
                start,
                end,
                filled,
                fixed_ratio,
            } => AnnotationShape::Circle {
                start: p(*start),
                end: p(*end),
                filled: *filled,
                fixed_ratio: *fixed_ratio,
            },
            AnnotationShape::Line { line, style } => AnnotationShape::Line {
                line: line.scaled(s),
                style: *style,
            },
            AnnotationShape::Redact { filter } => AnnotationShape::Redact {
                filter: filter.scaled(s),
            },
            AnnotationShape::Pen { points } => AnnotationShape::Pen {
                points: points.iter().map(|&q| p(q)).collect(),
            },
            AnnotationShape::Highlighter { points, flat_ends } => AnnotationShape::Highlighter {
                points: points.iter().map(|&q| p(q)).collect(),
                flat_ends: *flat_ends,
            },
            AnnotationShape::Text {
                frame,
                start,
                content,
                font_size,
                bold,
                italic,
                outline,
                appearance,
            } => AnnotationShape::Text {
                frame: frame.scaled(s),
                start: p(*start),
                content: content.clone(),
                font_size: *font_size,
                appearance: appearance.scaled(s),
                bold: *bold,
                italic: *italic,
                outline: *outline,
            },
        };
        // 1. 【标注】【比例映射】同步缩放已计算边界，避免导出时重新排版文字
        let bbox = Rect::from_ltrb(
            self.bbox.left() * s,
            self.bbox.top() * s,
            self.bbox.right() * s,
            self.bbox.bottom() * s,
        )
        .unwrap_or(self.bbox);

        Annotation {
            style_scale: self.style_scale * s,
            rotation_degrees: self.rotation_degrees,
            frozen_bounds: self.frozen_bounds.and_then(|bounds| {
                Rect::from_ltrb(
                    bounds.left() * s,
                    bounds.top() * s,
                    bounds.right() * s,
                    bounds.bottom() * s,
                )
            }),
            id: self.id,
            shape,
            color: self.color,
            shadow_color: self.shadow_color,
            stroke_width: self.stroke_width * s,
            bbox,
        }
    }
    /// 【标注】【位置移动】按逻辑增量 dx、dy 原地移动标注并更新边界；无返回值
    pub fn translate_mut(&mut self, dx: f32, dy: f32) {
        match &mut self.shape {
            AnnotationShape::ColorSample { sample } => {
                *sample = sample.mapped(|point| (point.0 + dx, point.1 + dy))
            }
            AnnotationShape::Number { mark } => {
                *mark = mark.translated((dx, dy));
            }
            AnnotationShape::Magnifier { magnifier } => {
                *magnifier = magnifier.translated((dx, dy));
            }
            AnnotationShape::Ruler { measurement } => {
                *measurement = measurement.mapped(|point| (point.0 + dx, point.1 + dy));
            }
            AnnotationShape::Arrow { line, .. } | AnnotationShape::Line { line, .. } => {
                *line = line.mapped(|point| (point.0 + dx, point.1 + dy));
            }
            AnnotationShape::NumeratedArrow { start, end, .. }
            | AnnotationShape::Rectangle { start, end, .. }
            | AnnotationShape::Marker { start, end, .. }
            | AnnotationShape::Circle { start, end, .. } => {
                start.0 += dx;
                start.1 += dy;
                end.0 += dx;
                end.1 += dy;
            }
            AnnotationShape::Redact { filter } => {
                *filter = filter.translated((dx, dy));
            }
            AnnotationShape::Pen { points } | AnnotationShape::Highlighter { points, .. } => {
                for p in points.iter_mut() {
                    p.0 += dx;
                    p.1 += dy;
                }
            }
            AnnotationShape::Text { start, frame, .. } => {
                start.0 += dx;
                start.1 += dy;
                *frame = frame.translated((dx, dy));
            }
        }
        // 1. 【标注】【位置移动】文字只移动现有边界，避免依赖字体系统
        match &self.shape {
            AnnotationShape::Text { .. } => {
                self.bbox = Rect::from_ltrb(
                    self.bbox.left() + dx,
                    self.bbox.top() + dy,
                    self.bbox.right() + dx,
                    self.bbox.bottom() + dy,
                )
                .unwrap_or(self.bbox);
            }
            _ => self.update_bbox(),
        }
    }
    /// 【标注】【尺寸变换】将坐标映射到 new_bbox，保留形状属性；返回新标注，文字需要调用方重新排版
    pub fn resize_to_bbox(&self, new_bbox: SignedRect) -> Annotation {
        let orig = self.bbox;

        let sx = if orig.width() > 0.0 {
            new_bbox.width() / orig.width()
        } else {
            1.0
        };
        let sy = if orig.height() > 0.0 {
            new_bbox.height() / orig.height()
        } else {
            1.0
        };

        let remap = |p: (f32, f32)| -> (f32, f32) {
            (
                new_bbox.left + (p.0 - orig.left()) * sx,
                new_bbox.top + (p.1 - orig.top()) * sy,
            )
        };

        let shape = match &self.shape {
            AnnotationShape::ColorSample { sample } => AnnotationShape::ColorSample {
                sample: crate::features::color_sample::ColorSample {
                    size: sample.size * sx.abs().max(sy.abs()),
                    ..sample.mapped(remap)
                },
            },
            AnnotationShape::Number { mark } => AnnotationShape::Number {
                mark: mark.mapped(remap),
            },
            AnnotationShape::Magnifier { magnifier } => AnnotationShape::Magnifier {
                magnifier: magnifier.mapped(remap),
            },
            AnnotationShape::Ruler { measurement } => AnnotationShape::Ruler {
                measurement: measurement.mapped(remap),
            },
            AnnotationShape::NumeratedArrow { start, end, number } => {
                AnnotationShape::NumeratedArrow {
                    start: remap(*start),
                    end: remap(*end),
                    number: *number,
                }
            }
            AnnotationShape::Arrow {
                line,
                style,
                line_style,
            } => AnnotationShape::Arrow {
                line: line.mapped(remap),
                style: *style,
                line_style: *line_style,
            },
            AnnotationShape::Rectangle {
                start,
                end,
                filled,
                radius,
                fixed_ratio,
            } => AnnotationShape::Rectangle {
                start: remap(*start),
                end: remap(*end),
                filled: *filled,
                radius: *radius,
                fixed_ratio: *fixed_ratio,
            },
            AnnotationShape::Marker {
                start,
                end,
                kind,
                filled,
                radius,
            } => AnnotationShape::Marker {
                start: remap(*start),
                end: remap(*end),
                kind: *kind,
                filled: *filled,
                radius: *radius,
            },
            AnnotationShape::Circle {
                start,
                end,
                filled,
                fixed_ratio,
            } => AnnotationShape::Circle {
                start: remap(*start),
                end: remap(*end),
                filled: *filled,
                fixed_ratio: *fixed_ratio,
            },
            AnnotationShape::Line { line, style } => AnnotationShape::Line {
                line: line.mapped(remap),
                style: *style,
            },
            AnnotationShape::Redact { filter } => AnnotationShape::Redact {
                filter: crate::features::filter::RegionFilter {
                    start: remap(filter.start),
                    end: remap(filter.end),
                    ..*filter
                },
            },
            AnnotationShape::Pen { points } => AnnotationShape::Pen {
                points: points.iter().map(|p| remap(*p)).collect(),
            },
            AnnotationShape::Highlighter { points, flat_ends } => AnnotationShape::Highlighter {
                points: points.iter().map(|p| remap(*p)).collect(),
                flat_ends: *flat_ends,
            },
            AnnotationShape::Text {
                frame,
                start,
                content,
                font_size,
                bold,
                italic,
                outline,
                appearance,
            } => {
                let scale = sx.abs().max(sy.abs());
                AnnotationShape::Text {
                    frame: frame.clone(),
                    start: remap(*start),
                    content: content.clone(),
                    font_size: crate::features::text::geometry::points(*font_size * scale),
                    appearance: appearance.clone(),
                    bold: *bold,
                    italic: *italic,
                    outline: *outline,
                }
            }
        };

        // 1. 【标注】【尺寸变换】更新非文字边界，文字尺寸由调用方排版
        let mut result = Annotation {
            shape,
            ..self.clone()
        };
        if let AnnotationShape::Number { mark } = result.shape {
            result.stroke_width = crate::features::number::clamped_width(
                self.stroke_width * sx.abs().max(sy.abs()),
                mark.style_scale,
            );
        }
        result.update_bbox();
        result
    }
}
