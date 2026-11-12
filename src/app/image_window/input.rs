//! 【图片窗口】【输入归属】标题栏和工具栏空白拖动交给窗口管理器，按钮和画布维持各自输入

use crate::{
    editor::EditorState,
    types::{MouseButton, image_window::Action},
    ui::image_window::{self, Hit},
};

/// 【图片窗口】【窗口尺寸】按 state 返回唯一图片窗口的逻辑尺寸
fn size(state: &EditorState) -> (u32, u32) {
    (
        state.placements[0].size.0 as u32,
        state.placements[0].size.1 as u32,
    )
}

/// 【图片窗口】【指针反馈】按 state 更新标题栏悬停，dirty 返回重绘；返回是否阻止画布处理
pub(crate) fn pointer(state: &mut EditorState, dirty: &mut u32) -> bool {
    if !state.image_window.enabled {
        return false;
    }
    let hit = image_window::hit(size(state), state.input.pointer.local);
    if state.image_window.hovered != hit {
        state.image_window.hovered = hit;
        crate::editor::dirty::mark_all_dirty(dirty, 1);
        state.damage_local(0, state.placements[0].rect().unwrap());
    }
    !canvas_gesture(state) && (hit.is_some() || state.image_window.pressed.is_some())
}

/// 【图片窗口】【画布持有】按 state 判断是否有从画布开始的活动手势，窗口边框不能中断其移动或释放
fn canvas_gesture(state: &EditorState) -> bool {
    state.input.mouse_down
        || state.input.left_canvas
        || state.tool_active
        || state.ann_drag.is_some()
        || state.navigation.pan.is_some()
        || state.navigation.slider.is_some()
        || state
            .text_session
            .as_ref()
            .is_some_and(|session| session.dragging)
}

/// 【图片窗口】【系统操作】按 state、button 和 pressed 设置待执行动作，dirty 返回重绘，返回是否消费本次按钮
pub(crate) fn button(
    state: &mut EditorState,
    button: MouseButton,
    pressed: bool,
    dirty: &mut u32,
) -> bool {
    if !state.image_window.enabled {
        return false;
    }
    if canvas_gesture(state) {
        return false;
    }
    let mut hit = image_window::hit(size(state), state.input.pointer.local);
    if hit.is_none() && state.toolbar.hit_test(state.input.pointer.local) == (true, None) {
        hit = Some(Hit::Move);
    }
    if button != MouseButton::Left {
        return pressed && hit.is_some();
    }
    if !pressed && let Some(owner) = state.image_window.pressed.take() {
        if Some(owner) == hit {
            state.image_window.action = match owner {
                Hit::Close => {
                    state.cancel = true;
                    None
                }
                Hit::Minimize => Some(Action::Minimize),
                Hit::Maximize => Some(Action::ToggleMaximize),
                _ => None,
            };
        }
        crate::editor::dirty::mark_all_dirty(dirty, 1);
        return true;
    }
    if pressed && let Some(hit) = hit {
        state.image_window.pressed = Some(hit);
        state.image_window.action = match hit {
            Hit::Move => Some(Action::Move),
            Hit::Resize(edge) => Some(Action::Resize(edge)),
            _ => None,
        };
        return true;
    }
    false
}
