//! 【截图起始】【输入交接】仅消费在顶部控件内开始的点击，画布手势维持原来的归属

use super::damage;
use crate::{
    editor::EditorState,
    features::recording::Format,
    types::{Mode, MouseButton},
    ui::capture_start::{Control, layout},
};

/// 【截图起始】【交互可用】按 state 返回起始控件是否接受输入，已选区或画布拖动时不拦截
fn available(state: &EditorState) -> bool {
    state.capture_start.visible
        && !state.input.mouse_down
        && !state.tool_active
        && state.selection.zone.is_none()
}

/// 【截图起始】【当前命中】按 state 返回当前显示器上指针命中的控件，起始控件不可用时返回空值
pub(crate) fn hit(state: &EditorState) -> Option<Control> {
    if !available(state) {
        return None;
    }
    layout::hit(
        state.placements[state.input.pointer.monitor_idx].size,
        state.capture_start.active(),
        state.input.pointer.local,
    )
}

/// 【截图起始】【指针移动】按 state 更新悬停并清除底层候选，dirty 返回重绘，返回是否消费移动
pub(crate) fn pointer(state: &mut EditorState, dirty: &mut u32) -> bool {
    let hovered = hit(state).map(|control| (state.input.pointer.monitor_idx, control));
    if state.capture_start.hovered != hovered {
        state.capture_start.hovered = hovered;
        damage(state, dirty);
    }
    hovered.is_some() || state.capture_start.pressed.is_some()
}

/// 【截图起始】【按钮归属】按 button、pressed 消费完整顶部点击序列，dirty 返回变化；跨区释放不触发底层选区
pub(crate) fn button(
    state: &mut EditorState,
    button: MouseButton,
    pressed: bool,
    dirty: &mut u32,
) -> bool {
    let current = hit(state).map(|control| (state.input.pointer.monitor_idx, control));
    if button != MouseButton::Left {
        return pressed && current.is_some();
    }
    if !pressed && let Some(owner) = state.capture_start.pressed.take() {
        if Some(owner) == current {
            activate(state, owner.1, dirty);
        }
        return true;
    }
    if pressed && current.is_some() {
        state.capture_start.pressed = current;
        crate::app::input::capture_actions::cancel(state);
        return true;
    }
    false
}

/// 【截图起始】【快捷键入口】按 action 触发与顶部按钮相同的录制动作，dirty 返回更新；起始控件不可用时忽略
pub(crate) fn shortcut(state: &mut EditorState, action: crate::keys::Action, dirty: &mut u32) {
    use crate::keys::Action;
    if !available(state) {
        return;
    }
    let control = match action {
        Action::RecordVideo => Control::Video,
        Action::RecordGif => Control::Gif,
        Action::RecordPause => Control::Pause,
        Action::RecordStop => Control::Stop,
        _ => return,
    };
    activate(state, control, dirty);
}

/// 【截图起始】【模式动作】按 control 切换截图或本次录制，dirty 返回更新，不保存全局录制格式
fn activate(state: &mut EditorState, control: Control, dirty: &mut u32) {
    match control {
        Control::Video | Control::Gif if state.capture_start.can_record() => {
            state.capture_start.recording = Some(if control == Control::Gif {
                Format::Gif
            } else {
                Format::Video
            });
            state.mode = Mode::Region;
            state.ui_hidden = true;
            crate::app::select_tool(state, crate::tools::Tool::Selection, dirty);
        }
        Control::Region => {
            state.capture_start.recording = None;
            state.mode = Mode::Editor;
            state.ui_hidden = false;
            crate::app::select_tool(state, crate::tools::Tool::Selection, dirty);
        }
        Control::Fullscreen => {
            state.capture_start.recording = None;
            state.mode = Mode::Editor;
            state.ui_hidden = false;
            let placement = &state.placements[state.input.pointer.monitor_idx];
            state.selection.zone = tiny_skia::Rect::from_xywh(
                placement.position.0 as f32,
                placement.position.1 as f32,
                placement.size.0 as f32,
                placement.size.1 as f32,
            );
            state.fullscreen_scope = true;
            crate::app::selection_history::record(state);
            super::sync(state, dirty);
            crate::app::select_tool(state, state.after_selection_tool, dirty);
        }
        Control::Pause if state.capture_start.can_pause() => {
            state
                .capture_start
                .query
                .command(if state.capture_start.status.phase == "paused" {
                    "resume"
                } else {
                    "pause"
                });
        }
        Control::Stop if state.capture_start.request_stop() => {
            state.capture_start.query.command("stop");
        }
        _ => return,
    }
    super::super::input::refresh_panels(state, dirty);
    damage(state, dirty);
}
