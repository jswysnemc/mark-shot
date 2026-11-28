//! 【取色标注】【指针拖动】复用标注编辑事务，只修改实际采样点，保持可见标签及其旋转不变

use crate::{
    editor::EditorState,
    types::{Annotation, AnnotationShape, SelectionHandle},
};

/// 【取色标注】【独立命中】按编辑器当前画布坐标返回最上层标注索引和指针或标签手势，引线与空白不接管输入
pub(super) fn hit(state: &EditorState) -> Option<(usize, SelectionHandle)> {
    let point = state.canvas_point();
    state
        .annotations
        .iter()
        .enumerate()
        .rev()
        .find_map(|(index, annotation)| {
            let AnnotationShape::ColorSample { sample } = annotation.shape else {
                return None;
            };
            if state.canvas_special_drag(annotation, point) == SelectionHandle::ColorSampleTip {
                return Some((index, SelectionHandle::ColorSampleTip));
            }
            let local = annotation.to_unrotated((point.0 as f32, point.1 as f32));
            crate::features::ruler::geometry::contains(sample.label_rect(), local)
                .then_some((index, SelectionHandle::Move))
        })
}

/// 【取色标注】【开始拖动】按 state 指针开始已有取色点的编辑事务，dirty 记录选择变化，返回是否接管本次按下
pub(crate) fn begin(state: &mut EditorState, dirty: &mut u32) -> bool {
    let Some((index, handle)) = hit(state) else {
        return false;
    };
    state.select_annotation_index(Some(index));
    crate::editor::annotation_drag::begin_drag_with_handle(state, index, handle);
    crate::editor::dirty::mark_all_dirty(dirty, state.placements.len());
    true
}

/// 【取色标注】【指针定位】将画布目标 target 映射到标签的局部坐标，只更新 annotation 的取色点和重绘范围
pub(crate) fn move_to(annotation: &mut Annotation, target: (f32, f32)) {
    let local = annotation.to_unrotated(target);
    let AnnotationShape::ColorSample { sample } = &mut annotation.shape else {
        return;
    };
    sample.source = local;
    annotation.update_bbox();
}
