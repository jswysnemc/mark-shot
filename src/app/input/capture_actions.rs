//! 【截图输入】【快捷手势】双击空白和中键单击共用动作，拖动不执行完成操作

use crate::{
    config::capture::ClickAction,
    editor::EditorState,
    interaction::DoubleClickTracker,
    keys::Action,
    types::{Finish, MouseButton},
};

#[derive(Default)]
pub(crate) struct Gestures {
    clicks: DoubleClickTracker<bool>,
    left_origin: Option<(f64, f64)>,
    middle: Option<Middle>,
}

struct Middle {
    origin: (f64, f64),
    action: ClickAction,
    dragged: bool,
}

/// 【截图输入】【选区内部】按 state 返回指针是否位于有效选区，包含全屏和图片编辑
fn inside(state: &EditorState) -> bool {
    state.selection.zone.is_some() && !super::selection_handles::outside(state)
}

/// 【截图输入】【位移阈值】按起点和终点返回是否已形成拖动，使用屏幕逻辑像素，不随画布缩放
fn dragged(origin: (f64, f64), point: (f64, f64)) -> bool {
    (point.0 - origin.0).hypot(point.1 - origin.1)
        > f64::from(crate::config::get().input.double_click_distance.max(4.0))
}

/// 【截图输入】【动作执行】按 action 执行 state 的复制、保存或关闭，dirty 返回绘制变化，返回是否有动作
pub(crate) fn execute(state: &mut EditorState, action: ClickAction, dirty: &mut u32) -> bool {
    let action = match action {
        ClickAction::None => return false,
        ClickAction::Copy => Action::Finish(Finish::Copy),
        ClickAction::Save => Action::Finish(Finish::Save),
        ClickAction::SaveAs => Action::SaveAs,
        ClickAction::Pin => Action::Finish(Finish::Pin),
        ClickAction::Cancel => {
            crate::editor::text_session::commit(state, dirty);
            state.cancel = true;
            return true;
        }
    };
    crate::app::actions::run(state, action, dirty);
    true
}

/// 【截图输入】【双击分派】按 button、pressed 和 action 观察空白处点击；返回是否已执行动作，保留文字与手柄优先级
pub(crate) fn button(
    state: &mut EditorState,
    button: MouseButton,
    pressed: bool,
    action: ClickAction,
    dirty: &mut u32,
) -> bool {
    if button != MouseButton::Left {
        state.input.capture_gestures.clicks.clear();
        state.input.capture_gestures.left_origin = None;
        return false;
    }
    if !pressed {
        state.input.capture_gestures.left_origin = None;
        return false;
    }
    let point = state.canvas_point();
    if !inside(state)
        || state.selected_tool == crate::tools::Tool::Ruler
        || state.text_session.is_some()
        || state.tool_active
        || state.input.mouse_down
        || super::selection_handles::hit(state).is_some()
        || state.annotations.iter().any(|ann| {
            state.canvas_body_hit(ann, point)
                || state.canvas_handle(ann, point) != crate::types::SelectionHandle::None
        })
    {
        state.input.capture_gestures.clicks.clear();
        return false;
    }
    let point = state.input.pointer.global;
    state.input.capture_gestures.left_origin = Some(point);
    if state
        .input
        .capture_gestures
        .clicks
        .register(true, (point.0 as f32, point.1 as f32))
    {
        state.input.capture_gestures.clicks.clear();
        state.end_collection_interaction();
        execute(state, action, dirty)
    } else {
        false
    }
}

/// 【截图输入】【拖动观测】按 state 的当前屏幕指针清除已拖动的双击序列，并返回中键是否可平移
pub(crate) fn moved(state: &mut EditorState) -> bool {
    observe(state, state.input.pointer.global)
}

/// 【截图输入】【轨迹范围】按 point 观测合并事件中的最远坐标，返回中键是否已经形成拖动
pub(crate) fn observe(state: &mut EditorState, point: (f64, f64)) -> bool {
    let gestures = &mut state.input.capture_gestures;
    if gestures
        .left_origin
        .is_some_and(|origin| dragged(origin, point))
    {
        gestures.clicks.clear();
        gestures.left_origin = None;
    }
    if let Some(middle) = &mut gestures.middle {
        middle.dragged |= dragged(middle.origin, point);
        return middle.dragged;
    }
    true
}

/// 【截图输入】【中键登记】按 action 为 state 登记单击候选；非独立按键或选区外保留纯平移
pub(crate) fn middle_press(state: &mut EditorState, action: ClickAction) {
    state.input.capture_gestures.middle = (action != ClickAction::None
        && inside(state)
        && !state.input.mouse_down
        && !state.input.left_canvas
        && !state.input.ctrl
        && !state.input.shift)
        .then_some(Middle {
            origin: state.input.pointer.global,
            action,
            dragged: false,
        });
    state.input.capture_gestures.clicks.clear();
}

/// 【截图输入】【中键释放】重验选区、按键及位移后执行原候选，dirty 返回变化；无候选或已拖动时无动作
pub(crate) fn middle_release(state: &mut EditorState, dirty: &mut u32) {
    let Some(middle) = state.input.capture_gestures.middle.take() else {
        return;
    };
    if !middle.dragged
        && !dragged(middle.origin, state.input.pointer.global)
        && inside(state)
        && !state.input.ctrl
        && !state.input.shift
        && !state.input.mouse_down
        && !state.input.left_canvas
    {
        execute(state, middle.action, dirty);
    }
}

/// 【截图输入】【手势取消】按 state 清除焦点中断或按键组合中的候选，不修改选区
pub(crate) fn cancel(state: &mut EditorState) {
    state.input.capture_gestures = Gestures::default();
}
