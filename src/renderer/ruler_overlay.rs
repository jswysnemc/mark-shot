//! 【尺子】【叠加绘制】最后绘制测量说明、临时十字线与选中控制点

use cosmic_text::{FontSystem, SwashCache, Weight};
use tiny_skia::{Color, LineCap, PathBuilder, Pixmap, Rect, Stroke, StrokeDash};

use super::text::TextStyle;
use super::{Painter, rect_bounds, scale_rect};
use crate::features::ruler::RulerGeometry;
use crate::features::ruler::drag;
use crate::features::ruler::geometry::{rect_center, rotated};
use crate::features::ruler::labels::InfoLabel;
use crate::features::ruler::state::RulerHover;
use crate::types::{Annotation, AnnotationShape};

/// 【尺子】【说明正文】将 label 的完整测量文字绘入 painter；支持窄范围多行，无返回值
fn label(painter: &mut Painter<'_>, label: &InfoLabel) {
    let scale = painter.scale();
    let Some(bounds) = scale_rect(&label.rect, scale).and_then(|rect| rect.round_out()) else {
        return;
    };
    let origin = (bounds.x() as f32 / scale, bounds.y() as f32 / scale);
    let Some(mut temporary) = Pixmap::new(bounds.width(), bounds.height()) else {
        return;
    };
    // 1. 【尺子】【说明正文】说明背景和字形使用固定像素原点，局部重绘不改变圆角采样
    {
        let mut local = painter.sub(&mut temporary).with_origin(origin);
        label_content(&mut local, label);
    }
    let position = painter.to_px(origin.0, origin.1);
    painter.draw_pixmap(
        position.0.round() as i32,
        position.1.round() as i32,
        &temporary,
    );
}

/// 【尺子】【说明内容】将 label 的圆角背景与全部文字绘入 painter 的固定原点缓冲；无返回值
fn label_content(painter: &mut Painter<'_>, label: &InfoLabel) {
    let rect = label.rect;
    painter.fill_rounded(
        rect,
        (9.0 * label.unit).min(rect.height() / 2.0),
        Color::from_rgba8(8, 13, 19, 225),
    );
    let padding = (10.0 * label.unit).min(rect.height() / 4.0);
    let height = (rect.height() - padding) / label.lines.len().max(1) as f32;
    for (index, line) in label.lines.iter().enumerate() {
        if let Some(line_rect) = Rect::from_xywh(
            rect.left(),
            rect.top() + padding / 2.0 + index as f32 * height,
            rect.width(),
            height,
        ) {
            painter.text(
                line_rect,
                line,
                TextStyle::new(label.font_size, Color::from_rgba8(229, 231, 235, 255))
                    .center()
                    .weight(Weight::SEMIBOLD),
            );
        }
    }
}

/// 【尺子】【全部说明绘制】按顺序把 labels 绘入 painter；导出在全部持久标注正文之后调用，无返回值
pub fn draw_labels(painter: &mut Painter<'_>, labels: &[InfoLabel]) {
    for item in labels {
        label(painter, item);
    }
}

/// 【尺子】【悬停绘制】将 hover 的临时十字线与原图坐标、颜色绘入 painter；不用于导出，无返回值
fn draw_hover(painter: &mut Painter<'_>, hover: &RulerHover) {
    let mut path = PathBuilder::new();
    path.move_to(hover.bounds.left(), hover.point.1);
    path.line_to(hover.bounds.right(), hover.point.1);
    path.move_to(hover.point.0, hover.bounds.top());
    path.line_to(hover.point.0, hover.bounds.bottom());
    if let Some(path) = path.finish() {
        painter.stroke_path(
            &path,
            Color::from_rgba8(45, 212, 191, 150),
            &Stroke {
                width: 1.0,
                dash: StrokeDash::new(vec![6.0, 4.0], 0.0),
                ..Stroke::default()
            },
        );
    }
    label(painter, &hover.label);
}

/// 【尺子】【局部叠加】把 labels 和可选 hover 绘入 canvas；origin、scale 映射坐标，dirty 限制局部区域，fonts、cache 提供字体，无返回值
pub fn draw_clipped(
    canvas: &mut Pixmap,
    labels: &[InfoLabel],
    hover: Option<&RulerHover>,
    origin: (f32, f32),
    scale: f32,
    dirty: Option<&Rect>,
    fonts: &mut FontSystem,
    cache: &mut SwashCache,
) {
    if labels.is_empty() && hover.is_none() {
        return;
    }
    let Some(area) = dirty
        .copied()
        .and_then(|rect| scale_rect(&rect, scale))
        .or_else(|| Rect::from_xywh(0.0, 0.0, canvas.width() as f32, canvas.height() as f32))
    else {
        return;
    };
    let Some((x, y, width, height)) = rect_bounds(&area, canvas.width(), canvas.height()) else {
        return;
    };
    let local_origin = (origin.0 + x as f32 / scale, origin.1 + y as f32 / scale);
    let Some(global) = Rect::from_xywh(
        local_origin.0,
        local_origin.1,
        width as f32 / scale,
        height as f32 / scale,
    ) else {
        return;
    };
    let visible = |item: &&InfoLabel| item.rect.intersect(&global).is_some();
    if !labels.iter().any(|item| visible(&item)) && hover.is_none() {
        return;
    }
    // 1. 【尺子】【局部叠加】说明背景和十字线半透明，只在刚恢复的区域绘制一次
    let Some(mut temporary) = Pixmap::new(width, height) else {
        return;
    };
    let mut painter = Painter::new(&mut temporary)
        .with_scale(scale)
        .with_origin(local_origin)
        .with_text(fonts, cache);
    for item in labels.iter().filter(visible) {
        label(&mut painter, item);
    }
    if let Some(hover) = hover {
        draw_hover(&mut painter, hover);
    }
    // 2. 【尺子】【局部叠加】只合成损坏范围，未清除像素不重复叠加背景或文字
    Painter::new(canvas).draw_pixmap(x as i32, y as i32, &temporary);
}

/// 【尺子】【选中控制框】按 annotation 绘制尺子操作框、旋转把手与定点；不修改测量数据，无返回值
pub(crate) fn draw_handles(painter: &mut Painter<'_>, annotation: &Annotation) {
    let AnnotationShape::Ruler { measurement } = annotation.shape else {
        return;
    };
    let Some(bounds) = measurement.unrotated_bounds() else {
        return;
    };
    let Some(frame) = drag::frame(measurement) else {
        return;
    };
    let color = crate::config::get().annotations.handles.color.get().color();
    let shadow = Color::from_rgba8(8, 13, 19, 230);
    let mut path = PathBuilder::new();
    if let Some(corners) = measurement.corners() {
        path.move_to(corners[0].0, corners[0].1);
        for point in &corners[1..] {
            path.line_to(point.0, point.1);
        }
        path.close();
    } else {
        path.push_rect(frame);
    }
    if let Some(path) = path.finish() {
        painter.stroke_path(
            &path,
            shadow,
            &Stroke {
                width: 3.2,
                ..Stroke::default()
            },
        );
        painter.stroke_path(
            &path,
            color,
            &Stroke {
                width: 1.6,
                dash: StrokeDash::new(vec![6.0, 4.0], 0.0),
                ..Stroke::default()
            },
        );
    }
    let center = rect_center(bounds);
    let rotation = match measurement.geometry {
        RulerGeometry::Length { rotation, .. } => rotation,
        _ => 0.0,
    };
    if let Some(handle) = drag::rotation_handle(measurement, annotation.stroke_width) {
        let top = rotated(
            (
                center.0,
                if matches!(measurement.geometry, RulerGeometry::Angle { .. }) {
                    frame.top()
                } else {
                    bounds.top() - 32.0 * measurement.unit(annotation.stroke_width)
                },
            ),
            center,
            rotation,
        );
        let mut path = PathBuilder::new();
        path.move_to(top.0, top.1);
        path.line_to(handle.0, handle.1);
        if let Some(path) = path.finish() {
            painter.stroke_path(
                &path,
                color,
                &Stroke {
                    width: 1.6,
                    line_cap: LineCap::Round,
                    ..Stroke::default()
                },
            );
        }
        painter.fill_circle(handle.0, handle.1, 7.0, shadow);
        painter.fill_circle(handle.0, handle.1, 5.5, color);
    }
    if matches!(measurement.geometry, RulerGeometry::Angle { .. }) {
        for (point, _) in drag::frame_handles(frame) {
            square_handle(painter, point, color, shadow);
        }
    }
    for (point, _) in drag::handles(measurement, drag::HANDLE_TOLERANCE) {
        if matches!(measurement.geometry, RulerGeometry::Angle { .. }) {
            painter.fill_circle(point.0, point.1, 7.0, shadow);
            painter.fill_circle(point.0, point.1, 5.5, color);
        } else {
            square_handle(painter, point, color, shadow);
        }
    }
}

/// 【尺子】【方形控制点】在 painter 中以 point 为中心绘制同一套 color、shadow 控制点；无返回值
fn square_handle(painter: &mut Painter<'_>, point: (f32, f32), color: Color, shadow: Color) {
    if let Some(rect) = Rect::from_xywh(point.0 - 6.0, point.1 - 6.0, 12.0, 12.0) {
        painter.fill_rounded(rect, 2.5, shadow);
    }
    if let Some(rect) = Rect::from_xywh(point.0 - 4.5, point.1 - 4.5, 9.0, 9.0) {
        painter.fill_rounded(rect, 2.0, color);
    }
}
