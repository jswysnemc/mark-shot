//! 【文字交互】【单击编辑】选择工具首次命中文字正文即显示输入光标

use crate::{
    editor::EditorState,
    types::{AnnotationShape, SelectionHandle},
};

/// 【文字交互】【编辑入口】按 state 中 index 命中的正文开启编辑；dirty 返回重绘范围，控制点仍交给拖动，返回是否消费点击
pub(crate) fn begin(state: &mut EditorState, index: usize, dirty: &mut u32) -> bool {
    let annotation = &state.annotations[index];
    if !matches!(annotation.shape, AnnotationShape::Text { .. })
        || !matches!(
            state.canvas_handle(annotation, state.canvas_point()),
            SelectionHandle::None | SelectionHandle::Move
        )
    {
        return false;
    }
    let id = annotation.id;
    state.ann_drag = None;
    crate::editor::text_session::begin_existing(state, id, dirty)
}
