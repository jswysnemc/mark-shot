//! 【标注多选】【共同变换】成员使用相同初始联合边界，文字存储尺寸与专用工具模型独立维护

use crate::{
    editor::TextState,
    features::{
        annotation_rotation::{geometry, resize},
        ruler::RulerGeometry,
        text_storage::transform as text_transform,
    },
    types::{Annotation, AnnotationShape, SelectionHandle},
};
use cosmic_text::Edit;
use tiny_skia::Rect;

/// 【标注多选】【放大镜同步】按annotation与canvas同步裁切后旋转轴心，再刷新正文bbox；其他形状保持自身边界，无返回值
pub(crate) fn sync_magnifier_geometry(annotation: &mut Annotation, canvas: Rect) {
    if !matches!(annotation.shape, AnnotationShape::Magnifier { .. }) {
        return;
    }
    // 1. 【标注多选】【轴心求值】描边宽度和原图裁切共同决定轴心，不能沿用修改前的存储值
    if let Some(pivot) = super::geometry::rotation_center(annotation, canvas)
        && let AnnotationShape::Magnifier { magnifier } = &mut annotation.shape
    {
        magnifier.pivot = pivot;
    }
    // 2. 【标注多选】【边界刷新】正文裁切与损坏区域依赖当前轴心，必须在模型同步之后更新
    annotation.update_bbox();
}

/// 【标注多选】【原图定点】按point、canvas与pixel返回最后原图像素以内的坐标，保留逻辑画布偏移
fn clamp(point: geometry::Point, canvas: Rect, pixel: f32) -> geometry::Point {
    (
        point
            .0
            .clamp(canvas.left(), (canvas.right() - pixel).max(canvas.left())),
        point
            .1
            .clamp(canvas.top(), (canvas.bottom() - pixel).max(canvas.top())),
    )
}

/// 【标注多选】【定点约束】限制annotation中生产points对应的定点到canvas，pixel为源图像素大小，无返回值
fn clamp_points(annotation: &mut Annotation, canvas: Rect, pixel: f32) {
    match &mut annotation.shape {
        AnnotationShape::Pen { points } | AnnotationShape::Highlighter { points, .. } => {
            for point in points {
                *point = clamp(*point, canvas, pixel);
            }
        }
        AnnotationShape::Arrow { line, .. } | AnnotationShape::Line { line, .. } => {
            *line = line.mapped(|point| clamp(point, canvas, pixel));
        }
        AnnotationShape::Number { mark } => {
            mark.tip = clamp(mark.tip, canvas, pixel);
            mark.bubble = clamp(mark.bubble, canvas, pixel);
        }
        AnnotationShape::Magnifier { magnifier } => {
            magnifier.source = clamp(magnifier.source, canvas, pixel);
        }
        AnnotationShape::Ruler { measurement }
            if matches!(measurement.geometry, RulerGeometry::Angle { .. }) =>
        {
            *measurement = measurement.mapped(|point| clamp(point, canvas, pixel));
        }
        AnnotationShape::Text { .. } => text_transform::clamp_anchor(annotation, canvas, pixel),
        _ => {}
    }
    annotation.update_bbox();
}

/// 【标注多选】【整组平移】按before、delta、canvas与pixel移动矩形和定点，返回新成员，源图与样式比例保持不变
fn moved(before: &Annotation, delta: geometry::Point, canvas: Rect, pixel: f32) -> Annotation {
    let mut annotation = before.clone();
    annotation.translate_mut(delta.0, delta.1);
    clamp_points(&mut annotation, canvas, pixel);
    annotation
}

/// 【标注多选】【共同旋转】按before、center、degrees、canvas与pixel公转成员轴心并自转，返回新成员，角度尺直接旋转定点
fn rotated(
    before: &Annotation,
    center: geometry::Point,
    degrees: f64,
    canvas: Rect,
    pixel: f32,
) -> Annotation {
    let mut annotation = before.clone();
    if let AnnotationShape::Ruler { measurement } = &mut annotation.shape
        && matches!(measurement.geometry, RulerGeometry::Angle { .. })
    {
        *measurement = measurement
            .mapped(|point| clamp(geometry::rotated(point, center, degrees), canvas, pixel));
        annotation.update_bbox();
        return annotation;
    }
    let Some(pivot) = super::geometry::rotation_center(before, canvas) else {
        return annotation;
    };
    let next = geometry::rotated(pivot, center, degrees);
    annotation = moved(before, (next.0 - pivot.0, next.1 - pivot.1), canvas, pixel);
    let added = |radians: f32| {
        geometry::normalized(f64::from(radians.to_degrees()) + degrees).to_radians() as f32
    };
    match &mut annotation.shape {
        AnnotationShape::Number { mark } => mark.rotation = added(mark.rotation),
        AnnotationShape::Magnifier { magnifier } => magnifier.rotation = added(magnifier.rotation),
        AnnotationShape::Redact { filter } => filter.rotation = added(filter.rotation),
        AnnotationShape::Ruler { measurement } => {
            if let RulerGeometry::Length { rotation, .. } = &mut measurement.geometry {
                *rotation = geometry::normalized(f64::from(*rotation) + degrees) as f32;
            }
        }
        _ => annotation.rotation_degrees = geometry::normalized(before.rotation_degrees + degrees),
    }
    annotation.update_bbox();
    annotation
}

/// 【标注多选】【文字映射】按annotation、before、old、target、handle、canvas、pixel与text重排存储矩形，返回值为空
fn resize_text(
    annotation: &mut Annotation,
    before: &Annotation,
    old: Rect,
    target: Rect,
    handle: SelectionHandle,
    canvas: Rect,
    pixel: f32,
    text: &mut TextState,
) {
    let wrapped = {
        let editor = crate::tools::text::ensure_text_editor(
            before,
            &mut text.editors,
            &mut text.font_system,
        );
        editor.with_buffer(|buffer| buffer.wrap() != cosmic_text::Wrap::None)
    };
    text_transform::resize(annotation, old, target, handle, canvas, pixel);
    if text_transform::corner(handle) {
        if !wrapped {
            crate::tools::text::fit_frame(annotation, None, &mut text.font_system);
        }
        if let AnnotationShape::Text { start, frame, .. } = &mut annotation.shape {
            frame.anchor = Some(*start);
        }
    }
    let editor = crate::tools::text::ensure_text_editor(
        annotation,
        &mut text.editors,
        &mut text.font_system,
    );
    crate::tools::text::update_text_bbox_inline(annotation, editor, &mut text.font_system);
}

/// 【标注多选】【共同缩放】按before、old、target、handle、canvas、pixel与text映射成员坐标，返回新成员，普通笔宽不随缩放变化
fn resized(
    before: &Annotation,
    old: Rect,
    target: Rect,
    handle: SelectionHandle,
    canvas: Rect,
    pixel: f32,
    text: &mut TextState,
) -> Annotation {
    let mut annotation = before.clone();
    if matches!(before.shape, AnnotationShape::Text { .. }) {
        resize_text(
            &mut annotation,
            before,
            old,
            target,
            handle,
            canvas,
            pixel,
            text,
        );
        return annotation;
    }
    let ratio = (
        f64::from(target.width()) / f64::from(old.width()),
        f64::from(target.height()) / f64::from(old.height()),
    );
    let map = |point: geometry::Point| {
        clamp(
            (
                (f64::from(target.left()) + (f64::from(point.0) - f64::from(old.left())) * ratio.0)
                    as f32,
                (f64::from(target.top()) + (f64::from(point.1) - f64::from(old.top())) * ratio.1)
                    as f32,
            ),
            canvas,
            pixel,
        )
    };
    match &mut annotation.shape {
        AnnotationShape::Rectangle { start, end, .. }
        | AnnotationShape::Circle { start, end, .. }
        | AnnotationShape::Marker { start, end, .. }
        | AnnotationShape::NumeratedArrow { start, end, .. } => {
            *start = map(*start);
            *end = map(*end);
        }
        AnnotationShape::Pen { points } | AnnotationShape::Highlighter { points, .. } => {
            for point in points {
                *point = map(*point);
            }
        }
        AnnotationShape::Arrow { line, .. } | AnnotationShape::Line { line, .. } => {
            *line = line.mapped(map)
        }
        AnnotationShape::Redact { filter } => {
            filter.start = map(filter.start);
            filter.end = map(filter.end);
        }
        AnnotationShape::Ruler { measurement } => *measurement = measurement.mapped(map),
        AnnotationShape::Magnifier { magnifier } => *magnifier = magnifier.mapped_storage(map),
        AnnotationShape::Number { mark } => {
            mark.tip = map(mark.tip);
            mark.bubble = map(mark.bubble);
            annotation.stroke_width = crate::features::number::clamped_width(
                annotation.stroke_width * ratio.0.max(ratio.1) as f32,
                mark.style_scale,
            );
        }
        AnnotationShape::Text { .. } => unreachable!(),
    }
    annotation.update_bbox();
    annotation
}

/// 【标注多选】【手势求值】按before、联合old、handle、start、current、canvas、pixel_scale、keep_aspect、snap与text返回本次成员，不累计前帧误差
pub(crate) fn dragged(
    before: &Annotation,
    old: Rect,
    handle: SelectionHandle,
    start: (f64, f64),
    current: (f64, f64),
    canvas: Rect,
    pixel_scale: f32,
    keep_aspect: bool,
    snap: bool,
    text: &mut TextState,
) -> Annotation {
    if !current.0.is_finite() || !current.1.is_finite() {
        return before.clone();
    }
    let pixel = 1.0 / pixel_scale.max(0.001);
    let point = clamp((current.0 as f32, current.1 as f32), canvas, pixel);
    let mut annotation = match handle {
        SelectionHandle::Move => {
            let delta = (
                (point.0 - start.0 as f32)
                    .clamp(canvas.left() - old.left(), canvas.right() - old.right()),
                (point.1 - start.1 as f32)
                    .clamp(canvas.top() - old.top(), canvas.bottom() - old.bottom()),
            );
            moved(before, delta, canvas, pixel)
        }
        SelectionHandle::Rotate => {
            let center = geometry::center(old);
            let Some(degrees) = geometry::drag_degrees(
                (f64::from(center.0), f64::from(center.1)),
                start,
                (f64::from(point.0), f64::from(point.1)),
                0.1 / f64::from(pixel_scale.max(0.001)),
            ) else {
                return before.clone();
            };
            rotated(
                before,
                center,
                if snap {
                    geometry::snapped(degrees)
                } else {
                    degrees
                },
                canvas,
                pixel,
            )
        }
        _ => resize::resized(old, handle, point, canvas, pixel_scale, keep_aspect).map_or_else(
            || before.clone(),
            |target| resized(before, old, target, handle, canvas, pixel, text),
        ),
    };
    sync_magnifier_geometry(&mut annotation, canvas);
    annotation
}
