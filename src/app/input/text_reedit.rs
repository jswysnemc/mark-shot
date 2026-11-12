//! 【文字交互】【双击编辑】首次命中文字正文保持对象选择，连续双击才显示输入光标

use crate::{
    editor::EditorState,
    types::{AnnotationShape, SelectionHandle},
};

/// 【文字交互】【编辑入口】按 state 中 index 登记正文点击，仅双击开启编辑；dirty 返回重绘范围，返回是否消费点击
pub(crate) fn begin(state: &mut EditorState, index: usize, dirty: &mut u32) -> bool {
    let annotation = &state.annotations[index];
    if !matches!(annotation.shape, AnnotationShape::Text { .. })
        || !matches!(
            state.canvas_handle(annotation, state.canvas_point()),
            SelectionHandle::None | SelectionHandle::Move
        )
    {
        state.input.clicks.clear();
        return false;
    }
    let id = annotation.id;
    let point = state.input.pointer.global;
    if !state.input.clicks.register(
        crate::interaction::ClickTarget::TextAnnotation(id),
        (point.0 as f32, point.1 as f32),
    ) {
        return false;
    }
    state.input.clicks.clear();
    state.ann_drag = None;
    crate::editor::text_session::begin_existing(state, id, dirty)
}
