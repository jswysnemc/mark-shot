//! 【多选区截图】【交互分派】独立接管冻结截图的手势，普通选区、标注和录制不共享临时遮罩

mod bar;
mod edit;
pub(super) mod preview;
pub(crate) use edit::State;
pub(super) use edit::{edit_hit, take_image};
#[cfg(test)]
mod tests;

use crate::{
    editor::EditorState,
    features::multi_capture::{
        Session,
        geometry::{self, Operation},
    },
    keys::{Action, Chord, Key},
    types::{Mode, MouseButton, Outputs},
};
use tiny_skia::Rect;

/// 【多选区截图】【重绘范围】按桌面 area 通知相交输出；无范围时恢复全部输出，dirty 返回显示器位图
fn damage(state: &mut EditorState, area: Option<Rect>, dirty: &mut u32) {
    for index in 0..state.placements.len() {
        let placement = &state.placements[index];
        let local = match area {
            Some(rect) => crate::tools::selection::global_selection_to_local(&rect, placement),
            None => placement.rect(),
        };
        if let Some(rect) = local {
            state.damage_local(index, rect);
            crate::editor::dirty::mark_dirty(dirty, index);
        }
    }
}

/// 【多选区截图】【进入模式】按 state 启用普通桌面截图的独立选区集合，dirty 登记重绘；不丢弃已有标注或文件编辑状态
pub(super) fn start(state: &mut EditorState, dirty: &mut u32) -> bool {
    if !matches!(state.mode, Mode::Editor | Mode::Region)
        || !state.capture_start.recording_controls
        || state.file_canvas.is_some()
        || state.fullscreen_scope
        || state.capture_start.active()
        || state.recording_tool.request.is_some()
        || state.tool_active
        || state.input.mouse_down
        || !state.annotations.is_empty()
        || state.pending.is_some()
        || state.text_session.is_some()
        || state.multi_capture.is_some()
        || state.upload.busy()
        || state.selection_translation.busy()
        || state.selection_translation.session.is_some()
        || state.selection_size.interacting()
    {
        return false;
    }
    let mut session = Session::default();
    session.confirmed.extend(state.selection.zone);
    state.multi_capture = Some(State::new(session));
    damage(state, None, dirty);
    true
}

/// 【多选区截图】【指针预览】按当前桌面指针更新活动草稿和新旧边界，返回是否由多选区会话接管
pub(super) fn motion(state: &mut EditorState, dirty: &mut u32) -> bool {
    let Some(selection) = state.multi_capture.as_mut() else {
        return false;
    };
    let session = &mut selection.regions;
    let old = session.marquee();
    let point = state.input.pointer.global;
    if let Some(bounds) = crate::utils::get_full_workspace_rect(&state.placements)
        && session.motion((point.0 as f32, point.1 as f32), bounds)
    {
        let area = old
            .into_iter()
            .chain(session.marquee())
            .reduce(|first, next| first.join(&next).unwrap_or(first));
        damage(state, area.and_then(|rect| rect.outset(3.0, 3.0)), dirty);
    }
    true
}

/// 【多选区截图】【按钮手势】左键添加、右键扣除；按 button 和 pressed 配对提交，返回是否拦截普通工具
pub(super) fn button(
    state: &mut EditorState,
    button: MouseButton,
    pressed: bool,
    dirty: &mut u32,
) -> bool {
    if edit::button(state, button, pressed, dirty) {
        return true;
    }
    let Some(selection) = state.multi_capture.as_mut() else {
        return false;
    };
    let session = &mut selection.regions;
    let operation = match button {
        MouseButton::Left => Operation::Add,
        MouseButton::Right => Operation::Subtract,
        _ => return true,
    };
    if pressed && session.operation().is_none() {
        let point = state.input.pointer.global;
        session.begin(operation, (point.0 as f32, point.1 as f32));
        edit::damage_buttons(state, dirty);
    } else if !pressed && session.operation() == Some(operation) {
        motion(state, dirty);
        let session = &mut state.multi_capture.as_mut().unwrap().regions;
        let area = session.marquee();
        session.commit();
        if let Some(area) = area {
            damage(state, area.outset(3.0, 3.0), dirty);
        }
        edit::damage_buttons(state, dirty);
    }
    true
}

/// 【多选区截图】【焦点中断】丢弃 state 中尚未松开的草稿，dirty 恢复此前已确认的预览
pub(super) fn cancel_drag(state: &mut EditorState, dirty: &mut u32) {
    if let Some(selection) = state.multi_capture.as_mut()
        && selection.cancel_gesture()
    {
        damage(state, None, dirty);
    }
}

/// 【多选区截图】【键盘操作】按 chord 进入编辑、直接导出或恢复区域；空选区和未提交草稿不能进入下一阶段
pub(super) fn key(state: &mut EditorState, chord: Option<Chord>, dirty: &mut u32) -> bool {
    let Some(session) = state.multi_capture.as_mut() else {
        return false;
    };
    let Some(chord) = chord else { return true };
    let action = crate::keys::map().lookup(chord);
    if chord.key == Key::Escape {
        escape(state, dirty);
    } else if matches!(action, Some(Action::Undo | Action::Redo)) {
        session.regions.history(action == Some(Action::Redo));
        damage(state, None, dirty);
    } else if session.ready() {
        if let Some(Action::Finish(target)) = action {
            state.selection.zone = geometry::bounds(&session.regions.confirmed);
            state.finish = Some(Outputs::from(target));
        } else if action == Some(Action::Accept) || chord.key == Key::Enter {
            edit::request(state);
        }
    }
    true
}

/// 【多选区截图】【返回单选】先取消未提交手势，否则退出独立截图模式；dirty 恢复普通编辑器画面
pub(super) fn escape(state: &mut EditorState, dirty: &mut u32) {
    if !state
        .multi_capture
        .as_mut()
        .is_some_and(State::cancel_gesture)
    {
        state.multi_capture = None;
        state.navigation.frames.fill(None);
    }
    damage(state, None, dirty);
}
