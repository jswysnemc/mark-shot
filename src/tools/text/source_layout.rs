//! 【文字标注】【原图排版】选框按原始字号测量，预览字形取整不改变存储矩形和旋转轴心

use crate::{
    features::text::geometry,
    types::{Annotation, AnnotationShape},
};
use cosmic_text::{Buffer, Edit, Editor, FontSystem, Metrics};
use tiny_skia::Rect;

/// 【文字标注】【原图编辑器】按annotation和fonts返回原图比例的独立标注与完整排版，不修改预览缓存
fn source_editor(annotation: &Annotation, fonts: &mut FontSystem) -> (Annotation, Editor<'static>) {
    let AnnotationShape::Text { appearance, .. } = &annotation.shape else {
        panic!("source layout requires a text annotation")
    };
    let source = annotation.scaled(1.0 / appearance.size_scale);
    let mut editor = Editor::new(Buffer::new_empty(Metrics::new(1.0, 1.0)));
    super::layout::synchronize_editor(&source, &mut editor, fonts, true);
    (source, editor)
}

/// 【文字标注】【原图自然宽度】按annotation和fonts返回无段内换行的最大字形宽度，使用原图字体行高
pub(super) fn natural_width(annotation: &Annotation, fonts: &mut FontSystem) -> f32 {
    let mut natural = annotation.clone();
    if let AnnotationShape::Text { frame, .. } = &mut natural.shape {
        frame.width = None;
        frame.height = None;
    }
    let (_, editor) = source_editor(&natural, fonts);
    editor.with_buffer(super::layout::natural_width)
}

/// 【文字标注】【原图选框】按annotation和fonts测量原图选框并映射到当前逻辑坐标，返回未旋转矩形
pub fn source_bounds(annotation: &Annotation, fonts: &mut FontSystem) -> Rect {
    let (source, editor) = source_editor(annotation, fonts);
    let bounds = editor.with_buffer(|buffer| {
        let metrics = buffer.metrics();
        let lines = crate::fonts::fixed_layout::lines(
            fonts,
            buffer,
            metrics.font_size,
            buffer.size().0,
            buffer.wrap(),
        );
        let mut width = lines.iter().map(|line| line.w).fold(0.0, f32::max);
        if buffer.wrap() != cosmic_text::Wrap::None {
            width = width.max(buffer.size().0.unwrap_or(0.0));
        }
        let base = source.text_stored_bounds().unwrap_or(source.bbox);
        geometry::bounds(
            (base.left(), base.top()),
            width,
            metrics.line_height * lines.len().max(1) as f32,
            1.0,
        )
    });
    let AnnotationShape::Text { appearance, .. } = &annotation.shape else {
        unreachable!()
    };
    Rect::from_xywh(
        bounds.left() * appearance.size_scale,
        bounds.top() * appearance.size_scale,
        bounds.width() * appearance.size_scale,
        bounds.height() * appearance.size_scale,
    )
    .unwrap()
}

/// 【文字标注】【实际段内换行】按editor返回是否有同一段落的多个布局行，显式换行的不同段落不计入
pub fn has_wrapped_paragraph(editor: &Editor<'static>) -> bool {
    editor.with_buffer(|buffer| {
        let mut previous = None;
        for run in buffer.layout_runs() {
            if previous == Some(run.line_i) {
                return true;
            }
            previous = Some(run.line_i);
        }
        false
    })
}

/// 【文字标注】【存储收框】按annotation、box_width和fonts收紧原始存储宽高，box_width为空时测量自然框，无返回值
pub fn fit_frame(annotation: &mut Annotation, box_width: Option<f32>, fonts: &mut FontSystem) {
    let base = annotation.text_stored_bounds().unwrap_or(annotation.bbox);
    if let AnnotationShape::Text { start, frame, .. } = &mut annotation.shape {
        *start = (base.left(), base.top());
        frame.width = box_width;
        frame.height = None;
        frame.painted = None;
    }
    let bounds = source_bounds(annotation, fonts);
    if let AnnotationShape::Text { frame, .. } = &mut annotation.shape {
        frame.width = Some(bounds.width());
        frame.height = Some(bounds.height());
    }
    annotation.bbox = bounds;
}

/// 【文字标注】【编辑换行宽度】按annotation和editor返回实际换行文档的逻辑框宽，没有段内换行返回空值
pub(crate) fn wrapped_box_width(annotation: &Annotation, editor: &Editor<'static>) -> Option<f32> {
    if !has_wrapped_paragraph(editor) {
        return None;
    }
    let AnnotationShape::Text { appearance, .. } = &annotation.shape else {
        return None;
    };
    editor.with_buffer(|buffer| {
        buffer
            .size()
            .0
            .map(|width| width + geometry::padding(appearance.size_scale).0 * 2.0)
    })
}
