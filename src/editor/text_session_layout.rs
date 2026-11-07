//! 【文字事务】【编辑收框】编辑使用可见选框宽度，提交按实际段内换行保存原始矩形

use crate::types::{Annotation, AnnotationShape};
use cosmic_text::{Editor, FontSystem};

/// 【文字事务】【已有编辑宽度】按annotation设置可见原图选框宽度，隐藏存储高度不限制编辑排版，无返回值
pub(super) fn begin_existing(annotation: &mut Annotation) {
    if let AnnotationShape::Text { frame, .. } = &mut annotation.shape {
        frame.width = Some(annotation.bbox.width());
        frame.height = Some(annotation.bbox.height());
        frame.painted = None;
    }
}

/// 【文字事务】【文档换行宽度】按annotation和editor记录提交前的实际段内换行框宽，显式换行返回空值
pub(super) fn wrapped_width(annotation: &Annotation, editor: &Editor<'static>) -> Option<f32> {
    crate::tools::text::wrapped_box_width(annotation, editor)
}

/// 【文字事务】【成稿矩形】按annotation、wrapped_width和fonts收紧存储宽高并更新首个定点，无返回值，保留身份与角度
pub(super) fn finish(
    annotation: &mut Annotation,
    wrapped_width: Option<f32>,
    fonts: &mut FontSystem,
) {
    crate::tools::text::fit_frame(annotation, wrapped_width, fonts);
    if let AnnotationShape::Text { start, frame, .. } = &mut annotation.shape {
        frame.anchor = Some(*start);
    }
}
