//! 【标注】【几何边界】统一计算逻辑边界、命中范围和重绘留量

use super::{Annotation, AnnotationShape};
use crate::theme::shadow;
use tiny_skia::Rect;

/// 【标注】【箭头头部】根据 stroke_width 与线段长度 len 计算头部长宽；返回长度与半宽，供绘制和边界共用
pub fn arrow_head(stroke_width: f32, len: f32) -> (f32, f32) {
    let arrow = &crate::config::get().annotations.arrow;
    let head_len = (stroke_width * arrow.head_length)
        .max(arrow.head_min)
        .min(len * arrow.head_max);
    (head_len, head_len * arrow.head_width)
}

impl Annotation {
    /// 【标注】【边界更新】重算当前形状的逻辑边界；文字边界由字体排版更新，无返回值
    pub fn update_bbox(&mut self) {
        match &self.shape {
            AnnotationShape::Number { mark } => {
                if let Some(bounds) = mark.body_bounds(self.stroke_width) {
                    self.bbox = bounds;
                }
            }
            AnnotationShape::Magnifier { magnifier } => {
                if let Some(bounds) = magnifier.bounds() {
                    self.bbox = bounds;
                }
            }
            AnnotationShape::Ruler { measurement } => {
                if let Some(bounds) = measurement.bounds() {
                    self.bbox = bounds;
                }
            }
            AnnotationShape::Rectangle { start, end, .. }
            | AnnotationShape::Marker { start, end, .. }
            | AnnotationShape::Circle { start, end, .. } => {
                let pad = self.stroke_width / 2.0 + shadow::width_bonus() / 2.0;
                self.bbox = Rect::from_ltrb(
                    start.0.min(end.0) - pad,
                    start.1.min(end.1) - pad,
                    start.0.max(end.0) + pad,
                    start.1.max(end.1) + pad,
                )
                .unwrap();
            }
            AnnotationShape::Arrow {
                line,
                style,
                line_style,
            } => {
                if let Some(bounds) = crate::features::arrow::path::outline(
                    line,
                    *style,
                    *line_style,
                    self.stroke_width,
                )
                .map(|path| path.bounds())
                .or_else(|| crate::features::line::geometry::anchor_bounds(line))
                {
                    self.bbox = bounds;
                }
            }
            AnnotationShape::Line { line, style } => {
                if let Some(bounds) =
                    crate::features::line::draw::outline(line, *style, self.stroke_width)
                        .map(|path| path.bounds())
                        .or_else(|| crate::features::line::geometry::anchor_bounds(line))
                {
                    self.bbox = bounds;
                }
            }

            AnnotationShape::NumeratedArrow { start, end, .. } => {
                let circle_radius =
                    self.stroke_width * crate::config::get().annotations.numbered.circle;
                let xs = [start.0 - circle_radius, start.0 + circle_radius, end.0];
                let ys = [start.1 - circle_radius, start.1 + circle_radius, end.1];

                self.bbox = Rect::from_ltrb(
                    xs.iter().cloned().fold(f32::INFINITY, f32::min),
                    ys.iter().cloned().fold(f32::INFINITY, f32::min),
                    xs.iter().cloned().fold(f32::NEG_INFINITY, f32::max),
                    ys.iter().cloned().fold(f32::NEG_INFINITY, f32::max),
                )
                .unwrap();
            }

            AnnotationShape::Redact { filter } => {
                if let Some(bounds) = filter.bounds() {
                    self.bbox = bounds;
                }
            }

            AnnotationShape::Pen { points } | AnnotationShape::Highlighter { points, .. } => {
                let min_x = points.iter().map(|p| p.0).fold(f32::INFINITY, f32::min);
                let min_y = points.iter().map(|p| p.1).fold(f32::INFINITY, f32::min);
                let max_x = points.iter().map(|p| p.0).fold(f32::NEG_INFINITY, f32::max);
                let max_y = points.iter().map(|p| p.1).fold(f32::NEG_INFINITY, f32::max);

                self.bbox = Rect::from_ltrb(min_x, min_y, max_x, max_y).unwrap();
            }

            AnnotationShape::Text { .. } => {}
        }
    }
    /// 【标注】【笔迹尾部】返回未缓存尾段的重绘区域，包含描边与辉光留量
    pub fn pen_active_tail_bbox(&self) -> Rect {
        let pad = crate::renderer::visual_pad(self.stroke_width);
        match &self.shape {
            AnnotationShape::Pen { points } | AnnotationShape::Highlighter { points, .. }
                if points.len() >= 3 =>
            {
                let p_prev = points[points.len() - 2];
                let p_last = points[points.len() - 1];
                let mid = ((p_prev.0 + p_last.0) / 2.0, (p_prev.1 + p_last.1) / 2.0);
                Rect::from_ltrb(
                    mid.0.min(p_last.0) - pad,
                    mid.1.min(p_last.1) - pad,
                    mid.0.max(p_last.0) + pad,
                    mid.1.max(p_last.1) + pad,
                )
                .unwrap_or(self.bbox)
            }
            AnnotationShape::Pen { points } | AnnotationShape::Highlighter { points, .. }
                if points.len() == 2 =>
            {
                let p0 = points[0];
                let p1 = points[1];
                Rect::from_ltrb(
                    p0.0.min(p1.0) - pad,
                    p0.1.min(p1.1) - pad,
                    p0.0.max(p1.0) + pad,
                    p0.1.max(p1.1) + pad,
                )
                .unwrap_or(self.bbox)
            }
            AnnotationShape::Pen { points } | AnnotationShape::Highlighter { points, .. }
                if points.len() == 1 =>
            {
                let p0 = points[0];
                Rect::from_ltrb(p0.0 - pad, p0.1 - pad, p0.0 + pad, p0.1 + pad).unwrap_or(self.bbox)
            }
            _ => self.damage_bbox(false),
        }
    }
    /// 【标注】【初步命中】判断全局 coordinates 是否处于可操作边界内；返回布尔值
    pub fn initial_hit_test(&self, coordinates: (f64, f64)) -> bool {
        if let Some(line) = self.shape.line() {
            let point = (coordinates.0 as f32, coordinates.1 as f32);
            let tolerance = crate::config::get().input.handle_hit_width * 0.5;
            return match self.shape {
                AnnotationShape::Arrow { style, .. } => crate::features::arrow::path::hit_test(
                    line,
                    style,
                    self.stroke_width,
                    point,
                    tolerance,
                ),
                _ => crate::features::line::geometry::insertion_at(
                    line,
                    point,
                    tolerance + self.stroke_width * 0.5,
                )
                .is_some(),
            };
        }
        if let AnnotationShape::Redact { filter } = self.shape {
            return crate::features::filter::drag::hit_test(
                filter,
                (coordinates.0 as f32, coordinates.1 as f32),
                false,
            ) != crate::types::SelectionHandle::None;
        }
        if let AnnotationShape::Number { mark } = self.shape {
            return crate::features::number::drag::hit_test(
                mark,
                self.stroke_width,
                (coordinates.0 as f32, coordinates.1 as f32),
                false,
            ) != crate::types::SelectionHandle::None;
        }
        if let AnnotationShape::Magnifier { magnifier } = self.shape {
            return crate::features::magnifier::drag::hit_test(
                magnifier,
                (coordinates.0 as f32, coordinates.1 as f32),
                false,
            ) != crate::types::SelectionHandle::None;
        }
        if let AnnotationShape::Ruler { measurement } = self.shape {
            return crate::features::ruler::drag::hit_test(
                measurement,
                (coordinates.0 as f32, coordinates.1 as f32),
                self.stroke_width,
                false,
                true,
            ) != crate::types::SelectionHandle::None;
        }
        let (x, y) = (coordinates.0 as f32, coordinates.1 as f32);
        let pad = crate::config::get().input.handle_hit_width;

        x >= self.bbox.left() - pad
            && x <= self.bbox.right() + pad
            && y >= self.bbox.top() - pad
            && y <= self.bbox.bottom() + pad
    }
    /// 【标注】【视觉留量】返回描边、阴影和文字外伸所需的逻辑边界留量
    pub fn visual_pad(&self) -> f32 {
        if let AnnotationShape::Redact { filter } = self.shape {
            return 2.0_f32.max(2.0 * filter.style_scale);
        }
        if let AnnotationShape::Number { mark } = self.shape {
            return 2.0 * mark.style_scale;
        }
        if let AnnotationShape::Magnifier { magnifier } = self.shape {
            return magnifier.border_width(self.stroke_width) / 2.0 + 2.0 * magnifier.style_scale;
        }
        if let AnnotationShape::Ruler { measurement } = self.shape {
            let digits = measurement
                .length_metrics()
                .map_or(1, |(width, height, _, _)| {
                    width.max(height).to_string().len()
                });
            return ((16.0 + digits as f32 * 8.0).max(34.0) * measurement.unit(self.stroke_width))
                .max(64.0 * measurement.style_scale);
        }
        let stroke_width = match &self.shape {
            AnnotationShape::Text {
                font_size,
                outline: Some(_),
                ..
            } => crate::tools::text::text_outline_stroke_width(*font_size),
            _ => self.stroke_width,
        };
        let letters_overhang = match &self.shape {
            AnnotationShape::Text { font_size, .. } => font_size * 0.25,
            _ => 0.0,
        };
        crate::renderer::visual_pad(stroke_width) + letters_overhang
    }
    /// 【标注】【重绘范围】根据 is_selected 决定是否包含操作框；返回完整逻辑重绘区域
    pub fn damage_bbox(&self, is_selected: bool) -> Rect {
        if let AnnotationShape::Redact { filter } = self.shape {
            let body = self
                .bbox
                .outset(self.visual_pad(), self.visual_pad())
                .unwrap_or(self.bbox);
            return if is_selected {
                crate::features::filter::drag::editing_bounds(filter)
                    .and_then(|editing| body.join(&editing))
                    .unwrap_or(body)
            } else {
                body
            };
        }
        if let Some(line) = self.shape.line() {
            let body = self
                .bbox
                .outset(self.visual_pad(), self.visual_pad())
                .unwrap_or(self.bbox);
            if is_selected
                && let Some(editing) = crate::features::line::drag::editing_bounds(
                    line,
                    self.bbox,
                    crate::renderer::selection_chrome_pad()
                        - crate::config::get().input.handle_hit_width * 0.5,
                )
            {
                return body.join(&editing).unwrap_or(body);
            }
            return body;
        }
        if let AnnotationShape::Number { mark } = self.shape {
            let body = self
                .bbox
                .outset(self.visual_pad(), self.visual_pad())
                .unwrap_or(self.bbox);
            if !is_selected {
                return body;
            }
            return crate::features::number::drag::controls(mark, self.stroke_width)
                .into_iter()
                .map(|(_, point)| point)
                .chain(crate::features::number::drag::rotation_handle(
                    mark,
                    self.stroke_width,
                ))
                .filter_map(|point| {
                    Rect::from_xywh(
                        point.0 - 8.0 * mark.style_scale,
                        point.1 - 8.0 * mark.style_scale,
                        16.0 * mark.style_scale,
                        16.0 * mark.style_scale,
                    )
                })
                .fold(body, |area, handle| area.join(&handle).unwrap_or(area));
        }
        if let AnnotationShape::Magnifier { magnifier } = self.shape {
            let padding = if is_selected {
                self.visual_pad().max(8.0 * magnifier.style_scale)
            } else {
                self.visual_pad()
            };
            let body = self.bbox.outset(padding, padding).unwrap_or(self.bbox);
            if is_selected
                && let Some(point) = crate::features::magnifier::drag::rotation_handle(magnifier)
                && let Some(handle) = Rect::from_xywh(
                    point.0 - 8.0 * magnifier.style_scale,
                    point.1 - 8.0 * magnifier.style_scale,
                    16.0 * magnifier.style_scale,
                    16.0 * magnifier.style_scale,
                )
            {
                return body.join(&handle).unwrap_or(body);
            }
            return body;
        }
        if let AnnotationShape::Ruler { measurement } = self.shape {
            let body = measurement
                .visual_bounds(measurement.unit(self.stroke_width))
                .and_then(|body| body.outset(2.0, 2.0))
                .unwrap_or(self.bbox);
            if !is_selected {
                return body;
            }
            let frame = crate::features::ruler::drag::frame(measurement)
                .and_then(|frame| frame.outset(8.0, 8.0));
            let handle =
                crate::features::ruler::drag::rotation_handle(measurement, self.stroke_width)
                    .and_then(|point| Rect::from_xywh(point.0 - 8.0, point.1 - 8.0, 16.0, 16.0));
            return [frame, handle]
                .into_iter()
                .flatten()
                .fold(body, |area, next| area.join(&next).unwrap_or(area));
        }
        let pad = if is_selected {
            crate::renderer::selection_chrome_pad().max(self.visual_pad())
        } else {
            self.visual_pad()
        };
        Rect::from_ltrb(
            self.bbox.left() - pad,
            self.bbox.top() - pad,
            self.bbox.right() + pad,
            self.bbox.bottom() + pad,
        )
        .unwrap_or(self.bbox)
    }
}
