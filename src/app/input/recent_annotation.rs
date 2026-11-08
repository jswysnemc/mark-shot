//! 【标注滚轮】【最近对象】完成绘制后首次滚轮直接调整刚创建的同类对象

use crate::{editor::EditorState, tools::Tool, types::AnnotationShape};

/// 【标注滚轮】【目标选择】按 state 当前工具匹配末个标注，dirty 返回选择重绘；手势、文字编辑和已有选择保持原路由，无返回值
pub(super) fn select(state: &mut EditorState, dirty: &mut u32) {
    if state.input.mouse_down
        || state.tool_active
        || state.pending.is_some()
        || state.ann_drag.is_some()
        || state.text_session.is_some()
        || !state.selected_annotation_ids().is_empty()
    {
        return;
    }
    let Some(annotation) = state.annotations.last() else {
        return;
    };
    let tool = match annotation.shape {
        AnnotationShape::ColorSample { .. } => Tool::Eyedropper,
        AnnotationShape::Rectangle { .. } => Tool::Rectangle,
        AnnotationShape::Circle { .. } => Tool::Circle,
        AnnotationShape::Arrow { .. } => Tool::Arrow,
        AnnotationShape::Line { .. } => Tool::Line,
        AnnotationShape::Pen { .. } => Tool::Pen,
        AnnotationShape::Highlighter { .. } => Tool::Highlighter,
        AnnotationShape::Text { .. } => Tool::Text,
        AnnotationShape::Marker { .. } => Tool::Marker,
        AnnotationShape::Number { .. } => Tool::Number,
        AnnotationShape::NumeratedArrow { .. } => Tool::NumeratedArrow,
        AnnotationShape::Magnifier { .. } => Tool::Magnifier,
        AnnotationShape::Redact { .. } => Tool::Redact,
        AnnotationShape::Ruler { .. } => Tool::Ruler,
    };
    if state.selected_tool != tool {
        return;
    }
    let id = annotation.id;
    super::select_tool(state, Tool::Pick, dirty);
    state.select_annotation_ids([id]);
    state.annotations_dirty = true;
    super::refresh_panels(state, dirty);
}
