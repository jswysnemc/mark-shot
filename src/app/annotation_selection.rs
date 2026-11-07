//! 【标注选框】【截图衔接】维护删除按钮的窗口位置、鼠标归属和损坏区域，绘制不进入导出

use crate::{
    editor::{EditorState, dirty::mark_dirty},
    features::annotation_selection::chrome,
    tools::Tool,
    types::{AnnotationShape, MouseButton},
};
use tiny_skia::Rect;

#[derive(Default)]
pub(crate) struct State {
    pub frame: Option<(usize, Rect)>,
    pub cache: Option<crate::ui::annotation_selection::delete_cache::Cache>,
    held: bool,
}

/// 【标注选框】【可见按钮】按 state 当前标注与 monitor 返回窗口逻辑位置，截图隐藏、文字工具和无选择时返回空值
fn rect(state: &EditorState, monitor: usize) -> Option<Rect> {
    if state.selected_tool != Tool::Pick
        || state.ui_hidden
        || state.selection_translation.busy()
        || state.upload.busy()
    {
        return None;
    }
    if state.selected_annotation_ids().len() > 1 {
        return super::annotation_collection::chrome::delete_rect(state, monitor);
    }
    let annotation = state
        .text_session
        .as_ref()
        .filter(|session| session.original_id.is_some())
        .map(|session| session.presented())
        .or_else(|| {
            state
                .pending
                .as_ref()
                .filter(|_| state.ann_drag.is_some())
                .or_else(|| {
                    state
                        .selected_annotation_index()
                        .and_then(|index| state.annotations.get(index))
                })
        })?;
    let placement = state.placements.get(monitor)?;
    state
        .canvas_local_rect(annotation.damage_bbox(false), monitor)?
        .intersect(&placement.rect()?)?;
    let mut frame = chrome::bounds(annotation)?;
    let mut radial = true;
    let degrees = match annotation.shape {
        AnnotationShape::Redact { filter } => filter.rotation,
        AnnotationShape::Number { mark } => mark.rotation,
        AnnotationShape::Magnifier { magnifier } => magnifier.rotation,
        AnnotationShape::Ruler { measurement } => match measurement.geometry {
            crate::features::ruler::RulerGeometry::Length { rotation, .. } => rotation,
            _ => {
                radial = false;
                frame = crate::features::ruler::drag::window_frame(
                    measurement,
                    state.hit_window_units(),
                )?;
                0.0
            }
        },
        _ => 0.0,
    };
    let corner = (frame.right(), frame.top());
    let corner = if annotation.uses_generic_rotation() {
        annotation.to_visual(corner)
    } else {
        crate::features::ruler::geometry::rotated(corner, chrome::center(frame), degrees)
    };
    let local = state.canvas_local_rect(frame, monitor)?;
    let corner = state
        .canvas_transform()
        .to_screen((f64::from(corner.0), f64::from(corner.1)));
    let offset = placement.offset();
    Some(chrome::delete_rect(
        local,
        (corner.0 as f32 - offset.0, corner.1 as f32 - offset.1),
        radial,
        (placement.size.0 as f32, placement.size.1 as f32),
    ))
}

/// 【标注选框】【位置同步】按 state 的活动输出登记旧新按钮范围，dirty 返回局部重绘，静止按钮不增加绘制
pub(super) fn sync(state: &mut EditorState, dirty: &mut u32) {
    let monitor = state.input.pointer.monitor_idx;
    let current = rect(state, monitor).map(|rect| (monitor, rect));
    let scale = state.placements[monitor].scale;
    let unchanged_cache = current.is_none()
        || current.is_some_and(|(_, rect)| {
            state
                .annotation_selection
                .cache
                .as_ref()
                .is_some_and(|cache| cache.matches(rect, scale))
        });
    if current != state.annotation_selection.frame || !unchanged_cache {
        for (monitor, rect) in state.annotation_selection.frame.into_iter().chain(current) {
            state.damage_local(monitor, rect.outset(2.0, 2.0).unwrap_or(rect));
            mark_dirty(dirty, monitor);
        }
        state.annotation_selection.frame = current;
        state.annotation_selection.cache = current.and_then(|(_, rect)| {
            crate::ui::annotation_selection::delete_cache::Cache::new(rect, scale)
        });
    }
}

/// 【标注选框】【删除命中】按 state 当前指针返回是否位于本次删除按钮，未选中或不在活动输出时为否
pub(crate) fn hovered(state: &EditorState) -> bool {
    rect(state, state.input.pointer.monitor_idx).is_some_and(|rect| {
        crate::ui::selection_size::state::contains(rect, state.input.pointer.local)
    })
}

/// 【标注选框】【左键删除】处理 state 已通过界面命中检查的左键按下，dirty 返回真实删除及旧按钮清理，返回是否消费
pub(super) fn press(state: &mut EditorState, dirty: &mut u32) -> bool {
    if state.ann_drag.is_some() || state.collection.drag.is_some() || !hovered(state) {
        return false;
    }
    state.annotation_selection.held = true;
    crate::editor::text_session::commit(state, dirty);
    crate::tools::pick::delete_annotation(state, dirty);
    super::input::refresh_panels(state, dirty);
    sync(state, dirty);
    true
}

/// 【标注选框】【释放归属】按 button 和 pressed 结束 state 中删除按钮的左键持有，返回是否消费，右键释放不结束左键
pub(super) fn release(state: &mut EditorState, button: MouseButton, pressed: bool) -> bool {
    if state.annotation_selection.held && !pressed && button == MouseButton::Left {
        state.annotation_selection.held = false;
        return true;
    }
    false
}
