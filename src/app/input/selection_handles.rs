//! 【选区把手】【统一输入】八个屏幕尺寸固定的控制点优先于画布工具，拖动期间保持原工具

use crate::{
    editor::EditorState,
    tools::{ToolBehavior, selection::SelectionTool},
    types::{CursorIcon, MouseButton, SelectionHandle},
};

/// 【选区把手】【命中查询】按 state 的画布坐标和视图比例返回八点命中；内部和普通边线返回 None
pub(crate) fn hit(state: &EditorState) -> Option<SelectionHandle> {
    let rect = state.selection.zone?;
    let point = state.canvas_point();
    let radius = 6.0 / state.canvas_transform().scale;
    let center = (
        f64::from(rect.left() + rect.width() / 2.0),
        f64::from(rect.top() + rect.height() / 2.0),
    );
    let left = f64::from(rect.left());
    let right = f64::from(rect.right());
    let top = f64::from(rect.top());
    let bottom = f64::from(rect.bottom());
    [
        (left, top, SelectionHandle::TopLeft),
        (right, top, SelectionHandle::TopRight),
        (left, bottom, SelectionHandle::BottomLeft),
        (right, bottom, SelectionHandle::BottomRight),
        (center.0, top, SelectionHandle::Top),
        (center.0, bottom, SelectionHandle::Bottom),
        (left, center.1, SelectionHandle::Left),
        (right, center.1, SelectionHandle::Right),
    ]
    .into_iter()
    .find(|(horizontal, vertical, _)| {
        (point.0 - horizontal).abs() <= radius && (point.1 - vertical).abs() <= radius
    })
    .map(|(_, _, handle)| handle)
}

/// 【选区把手】【区域判断】按 state 当前画布位置判断是否在选区外，未建立选区时返回假
pub(crate) fn outside(state: &EditorState) -> bool {
    let point = state.canvas_point();
    state.selection.zone.is_some_and(|rect| {
        point.0 < f64::from(rect.left())
            || point.0 > f64::from(rect.right())
            || point.1 < f64::from(rect.top())
            || point.1 > f64::from(rect.bottom())
    })
}

/// 【选区把手】【活动查询】按 state 返回是否有统一选区缩放手势，不把选区内部移动视为把手拖动
pub(super) fn active(state: &EditorState) -> bool {
    state.input.mouse_down
        && !matches!(
            state.selection.active_handle,
            SelectionHandle::None | SelectionHandle::Move
        )
}

/// 【选区把手】【按钮接管】按 button 和 pressed 开始或结束 state 的缩放，dirty 返回损坏；返回是否消费事件
pub(super) fn button(
    state: &mut EditorState,
    button: MouseButton,
    pressed: bool,
    dirty: &mut u32,
) -> bool {
    if button != MouseButton::Left {
        return false;
    }
    if !pressed && active(state) {
        SelectionTool.on_button(state, button, false, dirty);
        state.input.left_canvas = false;
        super::refresh_panels(state, dirty);
        return true;
    }
    if !pressed || state.input.mouse_down || (state.tool_active && state.text_session.is_none()) {
        return false;
    }
    let Some(handle) = hit(state) else {
        return false;
    };
    crate::editor::text_session::commit(state, dirty);
    state.end_collection_interaction();
    state.input.mouse_down = true;
    state.input.left_canvas = true;
    state.input.drag_start = None;
    state.tool_active = true;
    state.selection.creating = false;
    state
        .selection
        .set_drag(handle, Some(state.canvas_point()), state.selection.zone);
    true
}

/// 【选区把手】【拖动更新】按 state 当前画布位置更新选区并刷新面板，dirty 返回损坏；返回是否接管移动
pub(super) fn motion(state: &mut EditorState, dirty: &mut u32) -> bool {
    if !active(state) {
        return false;
    }
    SelectionTool.on_move(state, state.canvas_point(), dirty);
    super::refresh_panels(state, dirty);
    true
}

/// 【选区把手】【光标提示】按 state 的活动手势或八点命中返回缩放光标，没有命中返回空值
pub(super) fn cursor(state: &EditorState) -> Option<CursorIcon> {
    let handle = if active(state) {
        state.selection.active_handle
    } else {
        hit(state)?
    };
    crate::utils::cursor_for_handle(handle, active(state))
}
