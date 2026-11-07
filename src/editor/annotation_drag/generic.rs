//! 【普通标注】【编辑衔接】将纯手势结果交给标注事务，文字缩放使用原始字号与实际换行状态

use cosmic_text::Edit;

use crate::{
    editor::TextState,
    features::{annotation_rotation::drag, annotation_selection::chrome, text_storage::transform},
    types::{Annotation, AnnotationShape, SelectionHandle},
};

#[cfg(test)]
mod tests;

/// 【普通标注】【文字缩放】按annotation、before、handle、target、options和text映射存储矩形并重排字号，返回值为空
fn text_resize(
    annotation: &mut Annotation,
    before: &Annotation,
    handle: SelectionHandle,
    target: tiny_skia::Rect,
    options: drag::Options,
    text: &mut TextState,
) {
    if !matches!(before.shape, AnnotationShape::Text { .. }) {
        return;
    }
    let wrapped = {
        let editor = crate::tools::text::ensure_text_editor(
            before,
            &mut text.editors,
            &mut text.font_system,
        );
        editor.with_buffer(|buffer| buffer.wrap() != cosmic_text::Wrap::None)
    };
    *annotation = before.clone();
    transform::resize(
        annotation,
        chrome::bounds(before).unwrap_or(before.bbox),
        target,
        handle,
        options.bounds,
        1.0 / options.pixel_scale.max(0.001),
    );
    if transform::corner(handle) {
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

/// 【普通标注】【手势应用】按 annotation、before、handle、start、current、options 和 text 应用纯计算结果，返回是否属于通用手势
pub(super) fn apply(
    annotation: &mut Annotation,
    before: &Annotation,
    handle: SelectionHandle,
    start: (f64, f64),
    current: (f64, f64),
    options: drag::Options,
    text: &mut TextState,
) -> bool {
    if !before.uses_generic_rotation() {
        return false;
    }
    if let SelectionHandle::LinePoint(index) = handle
        && before.shape.line().is_some()
    {
        let pixel = 1.0 / options.pixel_scale.max(0.001);
        let clamp = |point: (f32, f32)| {
            (
                point
                    .0
                    .clamp(options.bounds.left(), options.bounds.right() - pixel),
                point
                    .1
                    .clamp(options.bounds.top(), options.bounds.bottom() - pixel),
            )
        };
        *annotation = before.clone();
        let local = clamp(before.to_unrotated(clamp((current.0 as f32, current.1 as f32))));
        annotation.shape.line_mut().unwrap().set_point(index, local);
        annotation.update_bbox();
        return true;
    }
    if let Some(mut change) = drag::dragged(before, handle, start, current, options) {
        if let Some(target) = change.resized {
            text_resize(
                &mut change.annotation,
                before,
                handle,
                target,
                options,
                text,
            );
        } else if handle == SelectionHandle::Move {
            transform::clamp_anchor(
                &mut change.annotation,
                options.bounds,
                1.0 / options.pixel_scale.max(0.001),
            );
            if matches!(&change.annotation.shape, AnnotationShape::Text { frame, .. } if frame.width==Some(0.0) || frame.height==Some(0.0))
            {
                let editor = crate::tools::text::ensure_text_editor(
                    &change.annotation,
                    &mut text.editors,
                    &mut text.font_system,
                );
                crate::tools::text::update_text_bbox_inline(
                    &mut change.annotation,
                    editor,
                    &mut text.font_system,
                );
            }
        }
        *annotation = change.annotation;
    }
    true
}
