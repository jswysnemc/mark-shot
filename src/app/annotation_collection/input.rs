//! 【标注多选】【输入归属】选择工具优先命中现有群组控制框，空白由单项选择入口开始框选

use crate::{editor::EditorState, types::MouseButton};

/// 【标注多选】【工具切换】按tool结束state旧工具手势，同一选择工具保留成员集合，dirty返回损坏，无返回值
pub(crate) fn deactivate(state: &mut EditorState, tool: crate::tools::Tool, dirty: &mut u32) {
    if state.selected_tool == crate::tools::Tool::Pick && tool == crate::tools::Tool::Pick {
        state.end_collection_interaction();
        crate::editor::annotation_drag::commit_drag_if_changed(state);
    } else {
        crate::tools::dispatch_deactivate(state.selected_tool, state, dirty);
    }
}

/// 【标注多选】【焦点中断】结束state仍未释放的框选或共同拖动，保留已更新结果，dirty返回损坏，无返回值
pub(crate) fn focus_lost(state: &mut EditorState, dirty: &mut u32) {
    if state.end_collection_interaction() {
        state.input.left_canvas = false;
        state.input.mouse_down = false;
        crate::editor::dirty::apply_damage_rects(state, dirty);
    }
}

/// 【标注多选】【鼠标按钮】处理state中button与pressed的群组归属，dirty返回局部损坏，返回是否消费
pub(crate) fn button(
    state: &mut EditorState,
    button: MouseButton,
    pressed: bool,
    dirty: &mut u32,
) -> bool {
    if button != MouseButton::Left {
        return false;
    }
    if !pressed {
        let handled = state.finish_collection_marquee() || state.finish_collection_drag();
        if handled {
            crate::editor::dirty::apply_damage_rects(state, dirty);
        }
        return handled;
    }
    if state.selected_annotation_ids().len() < 2 {
        return false;
    }
    let Some(bounds) = state.collection_bounds() else {
        return false;
    };
    let handle = state.canvas_collection_handle(bounds, state.canvas_point());
    if handle == crate::types::SelectionHandle::None {
        return false;
    }
    state.begin_collection_drag(handle);
    true
}

/// 【标注多选】【指针移动】处理state中global位置的框选或共同变换，dirty返回损坏，返回是否消费
pub(crate) fn pointer(state: &mut EditorState, global: (f64, f64), dirty: &mut u32) -> bool {
    if state.collection.marquee.is_some() {
        state.update_collection_marquee(global);
    } else if state.collection.drag.is_some() {
        state.update_collection_drag(global);
    } else {
        return false;
    }
    crate::editor::dirty::apply_damage_rects(state, dirty);
    true
}

/// 【标注多选】【修饰键更新】对state中的当前群组手势重算等比或旋转吸附，dirty返回损坏，返回是否消费
pub(crate) fn modifiers(state: &mut EditorState, changed: bool, dirty: &mut u32) -> bool {
    if changed && state.collection.drag.is_some() {
        state.update_collection_drag(state.canvas_point());
        crate::editor::dirty::apply_damage_rects(state, dirty);
        return true;
    }
    false
}
