//! 【取色标注】【字号编辑】滚轮调整最近采样标签，颜色快照和取样点保持稳定

/// 【取色标注】【滚轮字号】按 event 调整 state 选中的颜色标签，dirty 返回重绘；非取色标注或零滚动返回 false
pub(crate) fn wheel(
    state: &mut crate::editor::EditorState,
    event: crate::types::ScrollEvent,
    dirty: &mut u32,
) -> bool {
    let Some(index) = state.selected_annotation_index() else {
        return false;
    };
    let crate::types::AnnotationShape::ColorSample { sample } = state.annotations[index].shape
    else {
        return false;
    };
    let steps = event.canvas_steps();
    if steps == 0.0 {
        return true;
    }
    let scale = state.annotations[index].style_scale;
    let next = (sample.size / scale + steps as f32).clamp(8.0, 48.0) * scale;
    if next != sample.size {
        state.push_undo();
        if let crate::types::AnnotationShape::ColorSample { sample } =
            &mut state.annotations[index].shape
        {
            sample.size = next;
        }
        state.annotations[index].update_bbox();
        state.annotations_dirty = true;
        crate::editor::dirty::mark_all_dirty(dirty, state.placements.len());
    }
    true
}
